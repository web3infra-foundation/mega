use std::{collections::HashMap, sync::Arc};

use api_model::buck2::{types::TargetStatusResponse, ws::WSTargetBuildStatusEvent};
use callisto::{sea_orm_active_enums::OrionTargetStatusEnum, target_build_status};
use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::service::target_build_status_service::{NewTargetStatusInput, TargetBuildStatusService};

#[derive(Hash, Eq, PartialEq, Clone)]
struct ActionKey {
    package: String,
    name: String,
    configuration: String,
    category: String,
    identifier: String,
    action: String,
}

#[derive(Clone)]
pub struct TargetStatusCache {
    /// task_id -> (ActionKey -> ActiveModel)
    inner: Arc<RwLock<HashMap<Uuid, HashMap<ActionKey, target_build_status::ActiveModel>>>>,
}

impl TargetStatusCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn insert_event(&self, event: WSTargetBuildStatusEvent) {
        let task_id = match Uuid::parse_str(&event.context.task_id) {
            Ok(id) => id,
            Err(_) => {
                tracing::error!("Invalid task_id: {}", event.context.task_id);
                return;
            }
        };

        let status = OrionTargetStatusEnum::from_ws_status(&event.target.new_status);
        let key = ActionKey {
            package: event.target.configured_target_package.clone(),
            name: event.target.configured_target_name.clone(),
            configuration: event.target.configured_target_configuration.clone(),
            category: event.target.category.clone(),
            identifier: event.target.identifier.clone(),
            action: event.target.action.clone(),
        };
        let active_model = build_active_model(task_id, event, status);

        let mut guard = self.inner.write().await;
        let task_map = guard.entry(task_id).or_default();
        task_map.insert(key, active_model);
    }

    pub async fn flush_all(&self) -> Vec<target_build_status::ActiveModel> {
        let mut guard = self.inner.write().await;
        let mut result = Vec::new();
        for (_, action_map) in guard.drain() {
            result.extend(action_map.into_values());
        }
        result
    }

    pub async fn auto_flush_loop(
        self,
        conn: DatabaseConnection,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let models = self.flush_all().await;
                    if models.is_empty() {
                        continue;
                    }
                    if let Err(e) = TargetBuildStatusService::upsert_batch(&conn, models).await {
                        tracing::error!("Auto flush failed: {:?}", e);
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("TargetStatusCache auto flush shutting down");
                    let models = self.flush_all().await;
                    if !models.is_empty() {
                        let _ = TargetBuildStatusService::upsert_batch(&conn, models).await;
                    }
                    break;
                }
            }
        }
    }
}

fn build_active_model(
    task_id: Uuid,
    event: WSTargetBuildStatusEvent,
    status: OrionTargetStatusEnum,
) -> target_build_status::ActiveModel {
    TargetBuildStatusService::new_active_model(NewTargetStatusInput {
        id: Uuid::new_v4(),
        task_id,
        target_package: event.target.configured_target_package,
        target_name: event.target.configured_target_name,
        target_configuration: event.target.configured_target_configuration,
        category: event.target.category,
        identifier: event.target.identifier,
        action: event.target.action,
        status,
    })
}

impl Default for TargetStatusCache {
    fn default() -> Self {
        Self::new()
    }
}

pub trait FromWsStatus {
    fn from_ws_status(status: &str) -> Self;
    fn as_str(&self) -> &str;
}

impl FromWsStatus for OrionTargetStatusEnum {
    fn from_ws_status(status: &str) -> Self {
        match status.trim().to_ascii_lowercase().as_str() {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "success" | "succeeded" => Self::Success,
            "failed" => Self::Failed,
            _ => Self::Pending,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Success => "SUCCESS",
            Self::Failed => "FAILED",
        }
    }
}

pub async fn targets_status_by_task_id(
    conn: &DatabaseConnection,
    task_id: &str,
) -> Result<Vec<TargetStatusResponse>, (axum::http::StatusCode, String)> {
    let task_uuid = Uuid::parse_str(task_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid task_id".to_string(),
        )
    })?;
    let targets = TargetBuildStatusService::fetch_by_task_id(conn, task_uuid)
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    if targets.is_empty() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "No target status found".to_string(),
        ));
    }
    Ok(targets
        .into_iter()
        .map(|t| TargetStatusResponse {
            id: t.id.to_string(),
            task_id: t.task_id.to_string(),
            package: t.target_package,
            name: t.target_name,
            configuration: t.target_configuration,
            category: t.category,
            identifier: t.identifier,
            action: t.action,
            status: t.status.as_str().to_owned(),
        })
        .collect())
}

pub async fn target_status_by_id(
    conn: &DatabaseConnection,
    target_id: &str,
) -> Result<TargetStatusResponse, (axum::http::StatusCode, String)> {
    let target_uuid = Uuid::parse_str(target_id).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid target_id '{}': {}", target_id, e),
        )
    })?;
    let target = TargetBuildStatusService::find_by_id(conn, target_uuid)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database query failed: {}", e),
            )
        })?
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Target not found".to_string(),
        ))?;

    Ok(TargetStatusResponse {
        id: target.id.to_string(),
        task_id: target.task_id.to_string(),
        package: target.target_package,
        name: target.target_name,
        configuration: target.target_configuration,
        category: target.category,
        identifier: target.identifier,
        action: target.action,
        status: target.status.as_str().to_string(),
    })
}
