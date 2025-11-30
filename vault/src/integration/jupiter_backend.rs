use async_trait::async_trait;
use jupiter::storage::Storage;

use libvault_core::storage::Backend;

pub struct JupiterBackend {
    ctx: Storage,
}

impl JupiterBackend {
    pub fn new(ctx: Storage) -> Self {
        JupiterBackend { ctx }
    }
}

#[async_trait]
impl Backend for JupiterBackend {
    async fn list(&self, prefix: &str) -> Result<Vec<String>, libvault_core::errors::RvError> {
        let service = self.ctx.vault_storage();
        let prefix = prefix.to_string();
        match service.list_keys(prefix).await {
            Ok(keys) => Ok(keys),
            Err(e) => {
                println!("list {e:?}");
                Err(libvault_core::errors::RvError::ErrAuthModuleDisabled)
            }
        }
    }

    async fn get(
        &self,
        key: &str,
    ) -> Result<Option<libvault_core::storage::BackendEntry>, libvault_core::errors::RvError> {
        let service = self.ctx.vault_storage();
        let key = key.to_string();
        match service.load(key).await {
            Ok(Some(model)) => {
                let entry = libvault_core::storage::BackendEntry {
                    key: model.key,
                    value: model.value,
                };
                Ok(Some(entry))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                println!("get {e:?}");
                Err(libvault_core::errors::RvError::ErrAuthModuleDisabled)
            }
        }
    }

    async fn put(
        &self,
        entry: &libvault_core::storage::BackendEntry,
    ) -> Result<(), libvault_core::errors::RvError> {
        let service = self.ctx.vault_storage();
        let entry_clone = entry.clone();
        match service.save(entry_clone.key, entry_clone.value).await {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("put {e:?}");
                Err(libvault_core::errors::RvError::ErrAuthModuleDisabled)
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), libvault_core::errors::RvError> {
        let service = self.ctx.vault_storage();
        let key = key.to_string();
        match service.delete(key).await {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("delete {e:?}");
                Err(libvault_core::errors::RvError::ErrAuthModuleDisabled)
            }
        }
    }
}
