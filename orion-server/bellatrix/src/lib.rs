pub mod orion_client;

use api_model::buck2::api::TaskBuildRequest;
use common::config::BuildConfig;

use crate::orion_client::OrionClient;

#[derive(Clone)]
pub struct Bellatrix {
    orion: OrionClient,
    build_config: BuildConfig,
}

impl Bellatrix {
    pub fn new(build_config: BuildConfig) -> Self {
        let orion = OrionClient::new(build_config.orion_server.clone());
        Self {
            orion,
            build_config,
        }
    }

    pub fn enable_build(&self) -> bool {
        self.build_config.enable_build
    }

    pub async fn on_post_receive(&self, req: TaskBuildRequest) -> anyhow::Result<String> {
        let task_id = self.orion.trigger_build(req).await?;
        Ok(task_id)
    }
}
