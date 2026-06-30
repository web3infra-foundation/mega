use std::sync::Arc;

use jupiter::redis::{ConnectionManager, init_connection};

/// Main application context for the Mono application.
#[derive(Clone)]
pub struct AppContext {
    pub storage: jupiter::storage::Storage,
    pub vault: vault::integration::vault_core::VaultCore,
    pub config: Arc<common::config::Config>,
    pub connection: ConnectionManager,
}

impl AppContext {
    pub async fn new(config: common::config::Config) -> Self {
        let config = Arc::new(config);

        let storage = jupiter::storage::Storage::new(config.clone())
            .await
            .expect("init monorepo storage err");
        let connection = init_connection(&config.redis).await;

        let storage_for_vault = storage.clone();
        let vault = vault::integration::vault_core::VaultCore::new(storage_for_vault).await;

        storage
            .mono_service
            .init_monorepo(&config.monorepo)
            .await
            .expect("init monorepo failed");

        Self {
            storage,
            vault,
            config,
            connection,
        }
    }

    pub fn wrapped_context(&self) -> Arc<Self> {
        Arc::new(self.clone())
    }
}
