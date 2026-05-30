use api_model::buck2::api::TaskBuildRequest;
use serde::Deserialize;

/// Response from Orion task handler containing the assigned task ID.
#[derive(Deserialize, Debug)]
pub struct TaskResponse {
    pub task_id: String,
}

#[derive(Clone)]
pub(crate) struct OrionTaskHttpClient {
    base_url: String,
    client: reqwest::Client,
}

impl OrionTaskHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        // Loopback connections must bypass any system/HTTP_PROXY env so tests
        // and local dev don't get black-holed by a corporate proxy.
        let use_direct_connection = base_url.starts_with("http://127.0.0.1")
            || base_url.starts_with("https://127.0.0.1")
            || base_url.starts_with("http://localhost")
            || base_url.starts_with("https://localhost")
            || base_url.starts_with("http://[::1]")
            || base_url.starts_with("https://[::1]");
        let client = if use_direct_connection {
            reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        } else {
            reqwest::Client::new()
        };

        Self { base_url, client }
    }

    pub async fn trigger_build(&self, req: TaskBuildRequest) -> anyhow::Result<String> {
        let url = format!("{}/v2/task", self.base_url);
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
