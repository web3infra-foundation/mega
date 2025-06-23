use jupiter::storage::Storage;
use rusty_vault::storage::Backend;
use tokio::runtime::Handle;

pub struct JupiterBackend {
    ctx: Storage,
    rt: Handle,
}

impl JupiterBackend {
    pub fn new(ctx: Storage) -> Self {
        let rt = tokio::runtime::Handle::current();
        JupiterBackend { ctx, rt }
    }
}

impl Backend for JupiterBackend {
    fn list(&self, prefix: &str) -> Result<Vec<String>, rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        self.rt.block_on(async move {
            match service.list_keys(prefix).await {
                Ok(keys) => Ok(keys),
                Err(_) => Err(rusty_vault::errors::RvError::ErrPhysicalBackendKeyInvalid),
            }
        })
    }

    fn get(
        &self,
        key: &str,
    ) -> Result<Option<rusty_vault::storage::BackendEntry>, rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        self.rt.block_on(async move {
            match service.load(key).await {
                Ok(model) => {
                    let entry = rusty_vault::storage::BackendEntry {
                        key: model.key,
                        value: model.value,
                    };
                    Ok(Some(entry))
                }
                Err(_) => Err(rusty_vault::errors::RvError::ErrPhysicalBackendKeyInvalid),
            }
        })
    }

    fn put(
        &self,
        entry: &rusty_vault::storage::BackendEntry,
    ) -> Result<(), rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        self.rt.block_on(async move {
            match service.save(&entry.key, entry.value.clone()).await {
                Ok(_) => Ok(()),
                Err(_) => Err(rusty_vault::errors::RvError::ErrPhysicalBackendKeyInvalid),
            }
        })
    }

    fn delete(&self, key: &str) -> Result<(), rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        self.rt.block_on(async move {
            match service.delete(key).await {
                Ok(_) => Ok(()),
                Err(_) => Err(rusty_vault::errors::RvError::ErrPhysicalBackendKeyInvalid),
            }
        })
    }
}
