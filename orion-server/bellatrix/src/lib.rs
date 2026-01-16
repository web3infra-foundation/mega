pub mod orion_client;

use common::config::BuildConfig;

use crate::orion_client::{OrionBuildRequest, OrionClient};

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

    pub async fn on_post_receive(&self, req: OrionBuildRequest) -> anyhow::Result<()> {
        self.orion.trigger_build(req).await?;
        Ok(())
    }
}
