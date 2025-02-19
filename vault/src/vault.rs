use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use lazy_static::lazy_static;
use rusty_vault::core::{Core, SealConfig};
use rusty_vault::errors::RvError;
use rusty_vault::logical::{Operation, Request, Response};
use rusty_vault::storage;
use rusty_vault::storage::barrier_aes_gcm;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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

/// Initialize the vault core, used in `lazy_static!`
fn init() -> CoreInfo {
    const CORE_KEY_FILE: &str = "core_key.json"; // where the core key is stored, like `root_token`
    let dir = PathBuf::from("/tmp/rusty_vault_pki_module"); // RustyVault files TODO configurable
    let core_key_path = dir.join(CORE_KEY_FILE);
    // let dir = env::temp_dir().join("rusty_vault_pki_module"); // TODO: 改成数据库？

    let inited = dir.exists();
    if !inited {
        assert!(fs::create_dir(&dir).is_ok());
    }

    let mut conf: HashMap<String, Value> = HashMap::new();
    conf.insert("path".to_string(), Value::String(dir.to_string_lossy().into_owned()));

    let backend = storage::new_backend("file", &conf).unwrap(); // file or database
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
                let unseal = core.unseal(key);
                assert!(unseal.is_ok());
                unsealed = unseal.unwrap();
            }
        }

        assert!(unsealed);
        println!("root_token: {:?}", root_token);
    }

    CoreInfo { core: c, token: root_token }
}

pub async fn read_api(core: &Core, token: &str, path: &str) -> Result<Option<Response>, RvError> {
    let mut req = Request::new(path);
    req.operation = Operation::Read;
    req.client_token = token.to_string();
    core.handle_request(&mut req).await // !Send
}

pub async fn write_api(
    core: &Core,
    token: &str,
    path: &str,
    data: Option<Map<String, Value>>,
) -> Result<Option<Response>, RvError> {
    let mut req = Request::new(path);
    req.operation = Operation::Write;
    req.client_token = token.to_string();
    req.body = data;

    let resp = core.handle_request(&mut req).await; // !Send
    println!("path: {}, req.body: {:?}", path, req.body);
    resp
}

/// Write a secret to the vault (k-v)
pub async fn write_secret(name: &str, data: Option<Map<String, Value>>) -> Result<Option<Response>, RvError> {
    // async_std: stop spread of `!Send` (RwLockReadGuard cross .await), for `tokio::spawn`
    async_std::task::block_on(write_api(&CORE.core.read().unwrap(), &CORE.token, &format!("secret/{}", name), data))
}

/// Read a secret from the vault (k-v)
pub async fn read_secret(name: &str) -> Result<Option<Response>, RvError> {
    // async_std: stop spread of `!Send` (RwLockReadGuard cross .await), for `tokio::spawn`
    async_std::task::block_on(read_api(&CORE.core.read().unwrap(), &CORE.token, &format!("secret/{}", name)))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::*;

    #[tokio::test]
    async fn test_secret() {
        // create secret
        let kv_data = json!({
            "foo": "bar",
            "id": "oqpXWgEhXa1WDqMWBnpUW4jvrxGqJKVuJATy4MSPdKNS",
        })
        .as_object()
        .unwrap()
        .clone();
        write_secret("keyInfo", Some(kv_data.clone())).await.unwrap();

        let secret = read_secret("keyInfo").await.unwrap().unwrap().data;
        assert_eq!(secret, Some(kv_data));
        println!("secret: {:?}", secret.unwrap());

        assert!(read_secret("foo").await.unwrap().is_none());
        assert!(read_api(&CORE.core.read().unwrap(), &CORE.token, "secret1/foo").await.is_err());
    }
}