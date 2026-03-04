use common::errors::MegaError;

use crate::storage::{
    base_storage::{BaseStorage, StorageConnector},
    cla_storage::ClaStorage,
};

#[derive(Clone)]
pub struct ClaService {
    pub cla_storage: ClaStorage,
}

impl ClaService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
            cla_storage: ClaStorage { base: base_storage },
        }
    }

    pub fn mock() -> Self {
        let mock = BaseStorage::mock();
        Self {
            cla_storage: ClaStorage { base: mock },
        }
    }

    pub async fn get_or_create_status(&self, username: &str) -> Result<bool, MegaError> {
        Ok(self
            .cla_storage
            .get_or_create_status(username)
            .await?
            .cla_signed)
    }

    pub async fn ensure_signed(&self, username: &str) -> Result<(), MegaError> {
        let is_signed = self.cla_storage.is_signed(username).await?;
        if !is_signed {
            return Err(MegaError::Other(
                "[code:403] CLA_NOT_SIGNED: You have not signed the CLA yet.".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn sign_cla(&self, username: &str) -> Result<(), MegaError> {
        self.cla_storage.sign(username).await?;
        Ok(())
    }
}
