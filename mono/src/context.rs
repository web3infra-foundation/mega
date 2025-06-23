use std::sync::Arc;

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
}

impl AppContext {
    /// Creates a new application context with the given configuration.
    pub async fn new(config: common::config::Config) -> Self {
        let config = Arc::new(config);
        let storage = jupiter::storage::Storage::new(config.clone()).await;
        let vault = vault::integration::vault_core::VaultCore::new(storage.clone());

        Self {
            storage,
            vault,
            config,
        }
    }
}
