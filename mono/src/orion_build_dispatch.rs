use std::sync::Arc;

use async_trait::async_trait;
use ceres::application::build_trigger::BuildDispatchPort;
use common::errors::MegaError;
use orion_client::OrionBuildClient;

/// Adapts [`OrionBuildClient`] to [`BuildDispatchPort`] for mono assembly.
pub struct OrionBuildDispatch {
    inner: Arc<OrionBuildClient>,
}

impl OrionBuildDispatch {
    pub fn new(inner: Arc<OrionBuildClient>) -> Self {
        Self { inner }
    }

    pub fn into_arc(self) -> Arc<dyn BuildDispatchPort> {
        Arc::new(self)
    }
}

#[async_trait]
impl BuildDispatchPort for OrionBuildDispatch {
    fn enable_build(&self) -> bool {
        self.inner.enable_build()
    }

    async fn dispatch_build(
        &self,
        req: api_model::buck2::api::TaskBuildRequest,
    ) -> Result<String, MegaError> {
        self.inner
            .on_post_receive(req)
            .await
            .map_err(|e| MegaError::Other(format!("Failed to dispatch build to Orion: {e}")))
    }
}
