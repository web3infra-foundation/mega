pub mod command;
pub mod crypt;
pub mod pgp_key;

use std::{
    fs,
    path::Path,
    sync::{Arc, RwLock},
};

use rusty_vault::{
    cli::config,
    core::{Core, InitResult, SealConfig},
    http::sys::InitResponse,
    storage::{barrier_aes_gcm, physical},
};

pub const WORK_DIR_PATH_DEFAULT: &str = "/tmp/.mega/rusty_vault";

pub fn init_rv_core() -> (Arc<RwLock<Core>>, String) {
    let path = Path::new(WORK_DIR_PATH_DEFAULT);
    let config_path = path.join("config.hcl");
    let secrets_path = path.join("secrets");
    if !path.exists() {
        fs::create_dir_all(WORK_DIR_PATH_DEFAULT).unwrap();
    }

    if !config_path.exists() {
        let hcl_config = r#"
        storage "file" {
          path    = "/tmp/.mega/rusty_vault/data"
        }

        listener "tcp" {
          address     = "127.0.0.1:8200"
        }

        api_addr = "http://127.0.0.1:8200"
        log_level = "debug"
        log_format = "{date} {req.path}"
        pid_file = "/tmp/rusty_vault.pid"
    "#;
        fs::write(&config_path, hcl_config).unwrap();
    }
    let config = config::load_config(config_path.to_str().unwrap()).unwrap();

    let (_, storage) = config.storage.iter().next().unwrap();

    let backend = physical::new_backend(storage.stype.as_str(), &storage.config).unwrap();

    let barrier = barrier_aes_gcm::AESGCMBarrier::new(Arc::clone(&backend));

    let c = Arc::new(RwLock::new(Core {
        physical: backend,
        barrier: Arc::new(barrier),
        ..Default::default()
    }));
    let token;
    {
        let mut core = c.write().unwrap();
        core.config(Arc::clone(&c), Some(config)).unwrap();

        let seal_config = SealConfig {
            secret_shares: 10,
            secret_threshold: 5,
        };

        let mut unsealed = false;
        let init_result = if core.inited().unwrap() {
            let init_response: InitResponse =
                serde_json::from_str(&fs::read_to_string(secrets_path).unwrap()).unwrap();
            let init_result = InitResult {
                secret_shares: init_response
                    .keys
                    .iter()
                    .map(|key| hex::decode(key).unwrap())
                    .collect(),
                root_token: init_response.root_token,
            };
            init_result
        } else {
            let result = core.init(&seal_config);
            let init_result = result.unwrap();
            let resp = InitResponse {
                keys: init_result.secret_shares.iter().map(hex::encode).collect(),
                root_token: init_result.root_token.clone(),
            };
            // TODO: need to find a way to secure preserve key later
            fs::write(secrets_path, serde_json::to_string(&resp).unwrap()).unwrap();
            init_result
        };
        token = init_result.root_token;
        for i in 0..seal_config.secret_threshold {
            let key = &init_result.secret_shares[i as usize];
            let unseal = core.unseal(key);
            assert!(unseal.is_ok());
            unsealed = unseal.unwrap();
        }
        assert!(unsealed);
    }
    (Arc::clone(&c), token)
}

