pub use api_model::buck2::{api::TaskBuildRequest, status::Status, types::ProjectRelativePath};
use serde::Deserialize;

/// Response from Orion task handler containing the assigned task ID.
#[derive(Deserialize, Debug)]
pub struct TaskResponse {
    pub task_id: String,
}

#[derive(Clone)]
pub(crate) struct OrionClient {
    base_url: String,
    client: reqwest::Client,
}

impl OrionClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Trigger a build on Orion and return the assigned task ID.
    pub async fn trigger_build(&self, req: TaskBuildRequest) -> anyhow::Result<String> {
        let url = format!("{}/task", self.base_url);
        tracing::info!("Try to trigger build with params:{:?}", req);
        let res = self.client.post(&url).json(&req).send().await?;
        if res.status().is_success() {
            let task_response: TaskResponse = res.json().await?;
            tracing::info!("Received task_id from Orion: {}", task_response.task_id);
            Ok(task_response.task_id)
        } else {
            tracing::error!("Failed to trigger build: {}", res.status());
            Err(anyhow::anyhow!("Failed to trigger build: {}", res.status()))
        }
    }
}
