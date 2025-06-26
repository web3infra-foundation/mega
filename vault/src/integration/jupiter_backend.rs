use jupiter::storage::Storage;
use rusty_vault::storage::Backend;
use tokio::runtime::Handle;

pub struct JupiterBackend {
    ctx: Storage,
}

impl JupiterBackend {
    pub fn new(ctx: Storage) -> Self {
        JupiterBackend { ctx }
    }
}

impl Backend for JupiterBackend {
    fn list(&self, prefix: &str) -> Result<Vec<String>, rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        let handle = Handle::current();
        let prefix = prefix.to_string();
        std::thread::spawn(move || {
            // Using Handle::block_on to run async code in the new thread.
            handle.block_on(async {
                match service.list_keys(prefix).await {
                    Ok(keys) => Ok(keys),
                    Err(e) => {
                        println!("list {:?}", e);
                        Err(rusty_vault::errors::RvError::ErrAuthModuleDisabled)
                    }
                }
            })
        })
        .join()
        .unwrap()
    }

    fn get(
        &self,
        key: &str,
    ) -> Result<Option<rusty_vault::storage::BackendEntry>, rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        let handle = Handle::current();
        let key = key.to_string();
        std::thread::spawn(move || {
            // Using Handle::block_on to run async code in the new thread.
            handle.block_on(async move {
                match service.load(key).await {
                    Ok(Some(model)) => {
                        let entry = rusty_vault::storage::BackendEntry {
                            key: model.key,
                            value: model.value,
                        };
                        Ok(Some(entry))
                    }
                    Ok(None) => Ok(None),
                    Err(e) => {
                        println!("get {:?}", e);
                        Err(rusty_vault::errors::RvError::ErrAuthModuleDisabled)
                    }
                }
            })
        })
        .join()
        .inspect_err(|e| {
            eprintln!("Error in JupiterBackend::get: {:?}", e);
        })
        .unwrap()
    }

    fn put(
        &self,
        entry: &rusty_vault::storage::BackendEntry,
    ) -> Result<(), rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        let handle = Handle::current();
        let entry_clone = entry.clone();
        std::thread::spawn(move || {
            // Using Handle::block_on to run async code in the new thread.
            handle.block_on(async move {
                match service.save(entry_clone.key, entry_clone.value).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        println!("put {:?}", e);
                        Err(rusty_vault::errors::RvError::ErrAuthModuleDisabled)
                    }
                }
            })
        })
        .join()
        .unwrap()
    }

    fn delete(&self, key: &str) -> Result<(), rusty_vault::errors::RvError> {
        let service = self.ctx.vault_storage();
        let handle = Handle::current();
        let key = key.to_string();
        std::thread::spawn(move || {
            // Using Handle::block_on to run async code in the new thread.
            handle.block_on(async move {
                match service.delete(key).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        println!("delete {:?}", e);
                        Err(rusty_vault::errors::RvError::ErrAuthModuleDisabled)
                    }
                }
            })
        })
        .join()
        .unwrap()
    }
}
