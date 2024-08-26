use std::{
    fs,
    path::Path,
    sync::{Arc, RwLock},
};

use rusty_vault::{
    cli::config::{self, Config},
    core::{Core, InitResult, SealConfig},
    http::sys::InitResponse,
    storage::{barrier_aes_gcm, physical},
};
use zeroize::Zeroizing;

pub mod command;
pub mod crypt;
pub mod pgp_key;

pub struct RustyVault {
    pub core: Arc<RwLock<Core>>,
    pub token: String,
}

pub const WORK_DIR_PATH_DEFAULT: &str = "/tmp/.mega/rusty_vault";

/// Loads the Core instance and configuration.
///
/// This function reads the configuration file, initializes the storage backend,
/// and creates an instance of the Core structure.
///
/// # Returns
///
/// Returns a tuple containing the Arc-wrapped RwLock of the Core instance
/// and the loaded configuration.
pub fn load_core(work_dir: &Path) -> (Arc<RwLock<Core>>, Config) {
    // Define the path to the working directory and the configuration file
    // let path = Path::new(WORK_DIR_PATH_DEFAULT);
    let config_path = work_dir.join("config.hcl");

    // Load the configuration from the specified path
    let config = config::load_config(config_path.to_str().unwrap()).unwrap();

    // Extract the storage configuration from the loaded configuration
    let (_, storage) = config.storage.iter().next().unwrap();

    // Initialize the storage backend based on the storage type and configuration
    let backend = physical::new_backend(storage.stype.as_str(), &storage.config).unwrap();

    // Create a new AESGCMBarrier instance using the initialized backend
    let barrier = barrier_aes_gcm::AESGCMBarrier::new(Arc::clone(&backend));

    // Create a new Core instance with the initialized backend and barrier
    let c = Arc::new(RwLock::new(Core {
        physical: backend,
        barrier: Arc::new(barrier),
        ..Default::default()
    }));

    // Return the tuple containing the Core instance and loaded configuration
    (c, config)
}

/// Initializes the Rusty Vault Core.
///
/// This function sets up the necessary configuration and initializes the Core.
/// It creates the required configuration files, initializes the Core with a seal configuration,
/// and writes the generated keys to a secrets file.
pub fn init_rv_core(work_dir: Option<&Path>) {
    // Define paths for configuration and secrets
    let path = work_dir.unwrap_or_else(|| Path::new(WORK_DIR_PATH_DEFAULT));
    // let path = ;
    let config_path = path.join("config.hcl");
    let secrets_path = path.join("secrets");

    // Create the working directory if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(path).unwrap();
    }

    // Initialize the default HCL configuration
    let hcl_config = format!(
        r#"
        storage "file" {{
            path    = "{}/data"
        }}
    
        listener "tcp" {{
            address     = "127.0.0.1:8200"
        }}
    
        api_addr = "http://127.0.0.1:8200"
        log_level = "debug"
        log_format = "{{date}} {{req.path}}"
        pid_file = "/tmp/rusty_vault.pid"
        "#,
        path.to_str().unwrap()
    );
    fs::write(config_path, hcl_config).unwrap();

    // Define seal configuration
    let seal_config = SealConfig {
        secret_shares: 10,
        secret_threshold: 5,
    };

    // Load the Core and configuration
    let (c, config) = load_core(path);
    let mut core = c.write().unwrap();

    // Configure the Core and initialize with the seal configuration
    core.config(Arc::clone(&c), Some(config)).unwrap();
    let result = core.init(&seal_config);
    let init_result = result.unwrap();

    // Create InitResponse and write to the secrets file
    let resp = InitResponse {
        keys: init_result.secret_shares.iter().map(hex::encode).collect(),
        root_token: init_result.root_token.clone(),
    };
    // TODO: Need to find a way to secure/preserve the key later
    fs::write(secrets_path, serde_json::to_string(&resp).unwrap()).unwrap();
}

/// Unseals the Rusty Vault Core by reading the previously stored secrets.
///
/// This function loads the configuration and secrets, initializes the Core, and performs an unseal
/// operation using the stored keys from the secrets file.
///
/// # Returns
/// Returns a tuple containing an Arc-wrapped RwLock of the Core and the root token obtained
/// after successful unsealing.
pub fn unseal_rv_core(work_dir: Option<&Path>) -> (Arc<RwLock<Core>>, String) {
    // Define paths for secrets
    let path = work_dir.unwrap_or_else(|| Path::new(WORK_DIR_PATH_DEFAULT));
    // let path = Path::new(WORK_DIR_PATH_DEFAULT);
    let secrets_path = path.join("secrets");

    // Load the Core and configuration
    let (c, config) = load_core(path);
    let mut core = c.write().unwrap();

    // Configure the Core
    core.config(Arc::clone(&c), Some(config)).unwrap();

    // Obtain the seal configuration and load the stored keys from the secrets file
    let seal_config = core.seal_config().unwrap();
    let init_response: InitResponse =
        serde_json::from_str(&fs::read_to_string(secrets_path).unwrap()).unwrap();
    let init_result = InitResult {
        secret_shares: Zeroizing::new(
            init_response
                .keys
                .iter()
                .map(|key| hex::decode(key).unwrap())
                .collect(),
        ),
        root_token: init_response.root_token,
    };

    // Unseal the Core using the loaded keys
    let token = init_result.root_token;
    let mut unsealed = false;
    for i in 0..seal_config.secret_threshold {
        let key = &init_result.secret_shares[i as usize];
        let unseal = core.unseal(key);
        assert!(unseal.is_ok());
        unsealed = unseal.unwrap();
    }
    assert!(unsealed);

    (Arc::clone(&c), token)
}
