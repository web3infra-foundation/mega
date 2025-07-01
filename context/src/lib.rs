use std::sync::Arc;
use jupiter::tests::test_storage;

/// This is the main application context for the Mono application.
/// It holds shared state and configuration for the application.
/// Including database connections, configuration settings, encrypted vault functions, etc.
#[derive(Clone)]
pub struct AppContext {
    /// The storage sub-context for the from jupiter abstract layer.
    pub storage: jupiter::storage::Storage,

    /// The vault core for managing encrypted data.
    pub vault: vault::integration::vault_core::VaultCore,

    /// The client for P2P communication.
    #[cfg(feature = "p2p")]
    pub client: gemini::p2p::client::P2PClient,

    /// The configuration settings for the application.
    pub config: Arc<common::config::Config>,
}

impl AppContext {
    /// Creates a new application context with the given configuration.
    pub async fn new(config: common::config::Config) -> Self {

        let config = Arc::new(config);
       
        let storage = jupiter::storage::Storage::new(config.clone()).await;
        
        let storage_for_vault = storage.clone();

        let vault = tokio::task::spawn_blocking(move || {
            vault::integration::vault_core::VaultCore::new(storage_for_vault)
        }).await.expect("VaultCore::new panicked");

        
        
        
        
        #[cfg(feature = "p2p")]
        let client = gemini::p2p::client::P2PClient::new(storage.clone(), vault.clone());

        storage
            .services
            .mono_storage
            .init_monorepo(&config.monorepo)
            .await;

        Self {
            storage,
            vault,
            config,
            #[cfg(feature = "p2p")]
            client,
        }

      
    }

    pub fn wrapped_context(&self) -> Arc<Self> {
        Arc::new(self.clone())
    }

    pub async fn mock(config: common::config::Config) -> Self {
        
        let config = Arc::new(config);

        // use Existing test method
        let storage = test_storage(config.base_dir.clone()).await;
        
        let storage_for_vault = storage.clone();
        let temp_dir = config.base_dir.clone().join("vault");
        let key_path = temp_dir.join("core_key.json");
        std::fs::create_dir_all(&temp_dir).expect("Mock: Failed to create vault dir");
        
        let vault = tokio::task::spawn_blocking(move || {
            vault::integration::vault_core::VaultCore::config(storage_for_vault, key_path)
        })
            .await
            .expect("VaultCore::config panicked");

        #[cfg(feature = "p2p")]
        let client = gemini::p2p::client::P2PClient::new(storage.clone(), vault.clone());

        Self {
            storage,
            vault,
            config,
            #[cfg(feature = "p2p")]
            client,
        }
    }
}

