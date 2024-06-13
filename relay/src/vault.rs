use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use lazy_static::lazy_static;
use rusty_vault::core::{Core, SealConfig};
use rusty_vault::storage::{barrier_aes_gcm, physical};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
struct CoreKey {
    secret_shares: Vec<Vec<u8>>,
    root_token: String,
}

#[derive(Clone)]
pub struct CoreInfo {
    pub core: Arc<RwLock<Core>>,
    pub token: String,
}

// coding in `lazy_static!` with copilot seems lagging, so using function `init` instead
lazy_static! {
    pub static ref CORE: CoreInfo = init();
}

/// Initialize the vault core
fn init() -> CoreInfo {
    const CORE_KEY_FILE: &str = "core_key.json";
    let dir = PathBuf::from("/tmp/rusty_vault_pki_module");
    let core_key_path = dir.join(CORE_KEY_FILE);
    // let dir = env::temp_dir().join("rusty_vault_pki_module"); // TODO: 改成数据库？

    let inited = dir.exists();
    if !inited {
        assert!(fs::create_dir(&dir).is_ok());
    }

    let mut conf: HashMap<String, Value> = HashMap::new();
    conf.insert("path".to_string(), Value::String(dir.to_string_lossy().into_owned()));

    let backend = physical::new_backend("file", &conf).unwrap(); // file or database
    let barrier = barrier_aes_gcm::AESGCMBarrier::new(Arc::clone(&backend));

    let c = Arc::new(RwLock::new(Core { physical: backend, barrier: Arc::new(barrier), ..Default::default() }));

    let root_token;
    {
        let mut core = c.write().unwrap();
        assert!(core.config(Arc::clone(&c), None).is_ok());

        let seal_config = SealConfig { secret_shares: 10, secret_threshold: 5 };

        let mut unsealed = false;
        if !inited {
            let result = core.init(&seal_config);
            assert!(result.is_ok());
            let init_result = result.unwrap();
            println!("init_result: {:?}", init_result);

            for i in 0..seal_config.secret_threshold {
                let key = &init_result.secret_shares[i as usize];
                let unseal = core.unseal(key);
                assert!(unseal.is_ok());
                unsealed = unseal.unwrap();
            }

            root_token = init_result.root_token;

            let core_key = CoreKey {
                secret_shares: Vec::from(&init_result.secret_shares[..]),
                root_token: root_token.clone(),
            };
            let file = fs::File::create(core_key_path).unwrap();
            serde_json::to_writer_pretty(file, &core_key).unwrap();
        } else {
            let file = fs::File::open(core_key_path).unwrap();
            let core_key: CoreKey = serde_json::from_reader(file).unwrap();
            root_token = core_key.root_token.clone();

            for i in 0..seal_config.secret_threshold {
                let key = &core_key.secret_shares[i as usize];
                let unseal = core.unseal(&key);
                assert!(unseal.is_ok());
                unsealed = unseal.unwrap();
            }
        }

        assert!(unsealed);
        println!("root_token: {:?}", root_token);
    }

    CoreInfo { core: c, token: root_token }
}