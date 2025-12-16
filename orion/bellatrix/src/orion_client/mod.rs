use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct OrionBuildRequest {
    pub change: String, 
    pub mr: String,
    pub repo: String,
}

#[derive(Clone)]
pub struct OrionClient {
    base_url: String,
    client: reqwest::Client,
}

impl OrionClient {
    pub fn new() -> Self {
        Self {
            base_url: "http://127.0.0.1:8004".into(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn trigger_build(&self, req: OrionBuildRequest) -> anyhow::Result<()> {
        tracing::info!("Try to trigger build with params:{:?}", req);
        let res = self.client.post(&self.base_url).json(&req).send().await?;
        if res.status().is_success() {
            Ok(())
        } else {
            tracing::error!("Failed to trigger build: {}", res.status());
            Err(anyhow::anyhow!("Failed to trigger build: {}", res.status()))
        }
    }
}
