use std::{collections::HashMap, sync::Arc};

use api_model::buck2::ws::WSTargetBuildStatusEvent;
use callisto::target_build_status;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    model::internal::target_build_status::NewTargetStatusInput,
    repository::target_build_status_repo::TargetBuildStatusRepo,
};

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

        let key = ActionKey {
            package: event.target.configured_target_package.clone(),
            name: event.target.configured_target_name.clone(),
            configuration: event.target.configured_target_configuration.clone(),
            category: event.target.category.clone(),
            identifier: event.target.identifier.clone(),
            action: event.target.action.clone(),
        };
        let active_model = TargetBuildStatusRepo::new_active_model(
            NewTargetStatusInput::from_ws_event(task_id, event),
        );

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
}

impl Default for TargetStatusCache {
    fn default() -> Self {
        Self::new()
    }
}
