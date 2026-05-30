//! Client for triggering Orion builds via `orion-server`'s task API.
//!
//! `OrionBuildClient` wraps an HTTP transport (`http_client::OrionTaskHttpClient`)
//! and exposes a small, opinionated surface tailored for `ceres`/`mono` callers
//! (e.g. `enable_build()`, `on_post_receive()`). Keep the abstraction narrow:
//! only methods that callers actually need belong here.

mod http_client;

use api_model::buck2::api::TaskBuildRequest;
use common::config::BuildConfig;

use crate::http_client::OrionTaskHttpClient;

pub use http_client::TaskResponse;

#[derive(Clone)]
pub struct OrionBuildClient {
    http: OrionTaskHttpClient,
    build_config: BuildConfig,
}

impl OrionBuildClient {
    pub fn new(build_config: BuildConfig) -> Self {
        let http = OrionTaskHttpClient::new(build_config.orion_server.clone());
        Self { http, build_config }
    }

    pub fn enable_build(&self) -> bool {
        self.build_config.enable_build
    }

    pub async fn on_post_receive(&self, req: TaskBuildRequest) -> anyhow::Result<String> {
        let task_id = self.http.trigger_build(req).await?;
        Ok(task_id)
    }
}
