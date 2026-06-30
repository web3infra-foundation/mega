use common::errors::MegaError;

use super::service::MonoApiService;
use crate::model::gpg::GpgKey;

impl MonoApiService {
    pub async fn add_gpg_key(&self, user_id: String, gpg_content: String) -> Result<(), MegaError> {
        self.storage
            .gpg_storage()
            .add_gpg_key(user_id, gpg_content)
            .await
    }

    pub async fn remove_gpg_key(&self, user_id: String, key_id: String) -> Result<(), MegaError> {
        self.storage
            .gpg_storage()
            .remove_gpg_key(user_id, key_id)
            .await
    }

    pub async fn list_user_gpg_keys(&self, user_id: String) -> Result<Vec<GpgKey>, MegaError> {
        let raw_keys = self
            .storage
            .gpg_storage()
            .list_user_gpg(user_id.clone())
            .await;
        Ok(raw_keys
            .into_iter()
            .flatten()
            .map(|k| GpgKey::from_stored(user_id.clone(), k))
            .collect())
    }
}
