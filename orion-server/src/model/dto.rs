use api_model::buck2::types::TaskPhase;
use chrono::Utc;
use sea_orm::prelude::DateTimeUtc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::scheduler::{WorkerInfo, WorkerStatus};

#[derive(Debug, Serialize, ToSchema)]
pub struct OrionClientInfo {
    pub client_id: String,
    pub hostname: String,
    pub orion_version: String,
    #[schema(value_type = String, format = "date-time")]
    pub start_time: DateTimeUtc,
    #[schema(value_type = String, format = "date-time")]
    pub last_heartbeat: DateTimeUtc,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub enum CoreWorkerStatus {
    Idle,
    Busy,
    Error,
    Lost,
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct OrionClientQuery {
    pub hostname: Option<String>,
    pub status: Option<CoreWorkerStatus>,
    pub phase: Option<TaskPhase>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrionClientStatus {
    pub core_status: CoreWorkerStatus,
    pub phase: Option<TaskPhase>,
    pub error_message: Option<String>,
}

impl OrionClientStatus {
    pub fn from_worker_status(worker: &WorkerInfo) -> Self {
        match &worker.status {
            WorkerStatus::Idle => Self {
                core_status: CoreWorkerStatus::Idle,
                phase: None,
                error_message: None,
            },
            WorkerStatus::Busy { phase, .. } => Self {
                core_status: CoreWorkerStatus::Busy,
                phase: phase.clone(),
                error_message: None,
            },
            WorkerStatus::Error(msg) => Self {
                core_status: CoreWorkerStatus::Error,
                phase: None,
                error_message: Some(msg.clone()),
            },
            WorkerStatus::Lost => Self {
                core_status: CoreWorkerStatus::Lost,
                phase: None,
                error_message: None,
            },
        }
    }
}

#[derive(ToSchema, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(ToSchema, Serialize)]
pub struct BuildTargetDTO {
    pub id: String,
    pub task_id: String,
    pub path: String,
    pub latest_state: String,
}

#[derive(ToSchema, Serialize)]
pub struct BuildEventDTO {
    pub id: String,
    pub task_id: String,
    pub retry_count: i32,
    pub exit_code: Option<i32>,
    pub log: Option<String>,
    pub log_output_file: String,
    pub start_at: String,
    pub end_at: Option<String>,
}

impl From<&callisto::build_events::Model> for BuildEventDTO {
    fn from(model: &callisto::build_events::Model) -> Self {
        Self {
            id: model.id.to_string(),
            task_id: model.task_id.to_string(),
            retry_count: model.retry_count,
            exit_code: model.exit_code,
            log: model.log.clone(),
            log_output_file: model.log_output_file.clone(),
            start_at: model.start_at.to_string(),
            end_at: model.end_at.map(|dt| dt.with_timezone(&Utc).to_string()),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct OrionTaskDTO {
    pub id: String,
    pub changes: Value,
    pub repo_name: String,
    pub cl: String,
    pub created_at: String,
}

impl From<&callisto::orion_tasks::Model> for OrionTaskDTO {
    fn from(model: &callisto::orion_tasks::Model) -> Self {
        Self {
            id: model.id.to_string(),
            changes: model.changes.clone(),
            repo_name: model.repo_name.clone(),
            cl: model.cl.clone(),
            created_at: model.created_at.with_timezone(&Utc).to_string(),
        }
    }
}

#[derive(ToSchema, Serialize)]
pub enum BuildStatus {
    Running,
    Completed,
    Failed,
}
