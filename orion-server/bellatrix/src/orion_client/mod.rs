use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct OrionBuildRequest {
    pub repo: String,
    pub buck_hash: String,
    pub buckconfig_hash: String,
    pub mr: String,
    pub args: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct OrionClient {
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

    pub async fn trigger_build(&self, req: OrionBuildRequest) -> anyhow::Result<()> {
        let url = format!("{}/task", self.base_url);
        tracing::info!("Try to trigger build with params:{:?}", req);
        let res = self.client.post(&url).json(&req).send().await?;
        if res.status().is_success() {
            Ok(())
        } else {
            tracing::error!("Failed to trigger build: {}", res.status());
            Err(anyhow::anyhow!("Failed to trigger build: {}", res.status()))
        }
    }
}
