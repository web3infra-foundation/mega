use std::ops::Deref;

use callisto::{
    bot_keys, bots,
    sea_orm_active_enums::{BotStatusEnum, PermissionScopeEnum},
};
use common::errors::MegaError;
use rsa::{
    RsaPrivateKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, IntoActiveModel};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct BotsStorage {
    pub base: BaseStorage,
}

impl Deref for BotsStorage {
    type Target = BaseStorage;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl BotsStorage {
    pub async fn register_bot(
        &self,
        name: &str,
        organization_id: Option<i64>,
        creator_user_id: i64,
        permission_scope: PermissionScopeEnum,
    ) -> Result<(bots::Model, String), MegaError> {
        // Create and insert a new bot record
        let bot = self
            .new_bot_model(
                name,
                organization_id,
                creator_user_id,
                permission_scope,
                BotStatusEnum::Enabled,
            )
            .await?;

        // Generate RSA key pair
        let mut rng = OsRng;
        let private_key =
            RsaPrivateKey::new(&mut rng, 2048).map_err(|e| MegaError::Other(e.to_string()))?;
        let public_key = private_key.to_public_key();

        // Convert keys to PEM format
        let private_pem = private_key
            .to_pkcs8_pem(Default::default())
            .map_err(|e| MegaError::Other(e.to_string()))?
            .to_string();
        let public_pem = public_key
            .to_public_key_pem(Default::default())
            .map_err(|e| MegaError::Other(e.to_string()))?
            .to_string();

        // Save bot keys to the database
        self.new_bot_key(bot.id, private_pem.clone(), public_pem)
            .await?;

        // 5. Optional: Initialize tokens / permissions / audit logs for future expansion
        // Example:
        // self.init_bot_permissions(bot.id, &mut tx).await?;

        // 6. Return the bot record and private key (returned only once)
        Ok((bot, private_pem))
    }

    pub async fn new_bot_model(
        &self,
        name: &str,
        organization_id: Option<i64>,
        creator_user_id: i64,
        permission_scope: PermissionScopeEnum,
        status: BotStatusEnum,
    ) -> Result<bots::Model, MegaError> {
        let model = bots::Model::new(
            name.to_owned(),
            organization_id,
            creator_user_id,
            permission_scope,
            status,
        );
        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;

        Ok(res)
    }

    pub async fn new_bot_key(
        &self,
        bot_id: i64,
        private_key: String,
        public_key: String,
    ) -> Result<bot_keys::Model, MegaError> {
        let model = bot_keys::Model::new(bot_id, private_key, public_key);

        let res = model
            .into_active_model()
            .insert(self.get_connection())
            .await?;

        Ok(res)
    }
}
