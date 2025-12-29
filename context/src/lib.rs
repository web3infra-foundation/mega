use std::sync::Arc;

use jupiter::redis::init_connection;
use redis::aio::ConnectionManager;

/// This is the main application context for the Mono application.
/// It holds shared state and configuration for the application.
/// Including database connections, configuration settings, encrypted vault functions, etc.
#[derive(Clone)]
pub struct AppContext {
    /// The storage sub-context for the from jupiter abstract layer.
    pub storage: jupiter::storage::Storage,

    /// The vault core for managing encrypted data.
    pub vault: vault::integration::vault_core::VaultCore,

    /// The configuration settings for the application.
    pub config: Arc<common::config::Config>,

    pub connection: ConnectionManager,
}

impl AppContext {
    /// Creates a new application context with the given configuration.
    pub async fn new(config: common::config::Config) -> Self {
        let config = Arc::new(config);

        let storage = jupiter::storage::Storage::new(config.clone())
            .await
            .expect("init monorepo storage err");
        let connection = init_connection(&config.redis).await;

        let storage_for_vault = storage.clone();
        let vault = vault::integration::vault_core::VaultCore::new(storage_for_vault).await;

        let stg = storage.mono_storage();
        let blobs = stg.init_monorepo(&config.monorepo).await;
        storage
            .git_service
            .put_objects(blobs)
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
