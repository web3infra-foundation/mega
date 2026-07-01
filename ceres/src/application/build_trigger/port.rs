use std::sync::Arc;

use api_model::buck2::api::TaskBuildRequest;
use async_trait::async_trait;
use common::errors::MegaError;

/// Dispatches build tasks to an external execution layer (e.g. Orion).
#[async_trait]
pub trait BuildDispatchPort: Send + Sync {
    fn enable_build(&self) -> bool;

    async fn dispatch_build(&self, req: TaskBuildRequest) -> Result<String, MegaError>;
}

pub type SharedBuildDispatch = Arc<dyn BuildDispatchPort>;
