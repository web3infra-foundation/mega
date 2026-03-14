use std::ops::Deref;

use callisto::{
    bot_installations, bot_keys, bots,
    sea_orm_active_enums::{
        BotStatusEnum, InstallationBotStatusEnum, InstallationTargetTypeEnum, PermissionScopeEnum,
    },
};
use common::errors::MegaError;
use rsa::{
    RsaPrivateKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};

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

    pub async fn get_installed_bot_by_id(
        &self,
        bot_id: i64,
    ) -> Result<Vec<bot_installations::Model>, MegaError> {
        let installations = bot_installations::Entity::find()
            .filter(bot_installations::Column::BotId.eq(bot_id))
            .all(self.get_connection())
            .await?;

        Ok(installations)
    }

    /// Install a bot to a target (organization or repository)
    pub async fn install_bot(
        &self,
        bot_id: i64,
        target_type: InstallationTargetTypeEnum,
        target_id: i64,
        installed_by: i64,
    ) -> Result<bot_installations::Model, MegaError> {
        // Check if already installed
        if (bot_installations::Entity::find()
            .filter(bot_installations::Column::BotId.eq(bot_id))
            .filter(bot_installations::Column::TargetType.eq(target_type.clone()))
            .filter(bot_installations::Column::TargetId.eq(target_id))
            .one(self.get_connection())
            .await?)
            .is_some()
        {
            return Err(MegaError::Other(
                "Bot already installed on this target".into(),
            ));
        }

        // Insert installation record
        let model = bot_installations::Model::new(
            bot_id,
            target_type,
            target_id,
            InstallationBotStatusEnum::Enabled,
            installed_by,
        );

        let active_model = model.into_active_model();

        let res = active_model.insert(self.get_connection()).await?;
        Ok(res)
    }

    /// Uninstall a bot from a target
    pub async fn uninstall_bot(
        &self,
        bot_id: i64,
        target_type: InstallationTargetTypeEnum,
        target_id: i64,
    ) -> Result<(), MegaError> {
        let installation = bot_installations::Entity::find()
            .filter(bot_installations::Column::BotId.eq(bot_id))
            .filter(bot_installations::Column::TargetType.eq(target_type.clone()))
            .filter(bot_installations::Column::TargetId.eq(target_id))
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other("Bot installation not found".into()))?
            .into_active_model();

        installation
            .into_active_model()
            .delete(self.get_connection())
            .await?;

        Ok(())
    }

    /// Change the status of an installed bot (Enabled / Disabled)
    pub async fn change_installed_bot_status(
        &self,
        bot_id: i64,
        target_type: InstallationTargetTypeEnum,
        target_id: i64,
        status: InstallationBotStatusEnum,
    ) -> Result<bot_installations::Model, MegaError> {
        let mut installation = bot_installations::Entity::find()
            .filter(bot_installations::Column::BotId.eq(bot_id))
            .filter(bot_installations::Column::TargetType.eq(target_type.clone()))
            .filter(bot_installations::Column::TargetId.eq(target_id))
            .one(self.get_connection())
            .await?
            .ok_or_else(|| MegaError::Other("Bot installation not found".into()))?
            .into_active_model();

        installation.status = Set(status);
        let res = installation.update(self.get_connection()).await?;
        Ok(res)
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
