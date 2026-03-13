use std::ops::Deref;

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use callisto::{
    bot_keys, bot_tokens, bots,
    sea_orm_active_enums::{BotStatusEnum, PermissionScopeEnum},
};
use chrono::Utc;
use common::errors::MegaError;
use hmac::{Hmac, Mac};
use idgenerator::IdInstance;
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
    RsaPrivateKey,
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait,
    EntityTrait,
    IntoActiveModel,
    QueryFilter,
    QueryOrder,
};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::Condition;
use sha2::Sha256;

use crate::{
    model::bot_token_dto::BotTokenInfo,
    storage::base_storage::{BaseStorage, StorageConnector},
};

const BOT_TOKEN_PREFIX: &str = "bot_";
const BOT_TOKEN_RANDOM_LEN: usize = 32;
const BOT_TOKEN_HMAC_KEY_ENV: &str = "MEGA_BOT_TOKEN_HMAC_SECRET";

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

    /// Generate a new bot token, persist its HMAC-SHA256 hash and return the model with plaintext.
    pub async fn generate_bot_token(
        &self,
        bot_id: i64,
        token_name: &str,
        expires_at: Option<DateTimeWithTimeZone>,
    ) -> Result<(bot_tokens::Model, String), MegaError> {
        let token_plain = generate_bot_token_plain()?;
        let token_body = token_plain
            .strip_prefix(BOT_TOKEN_PREFIX)
            .unwrap_or(&token_plain);

        let hmac_key = load_bot_token_hmac_key()?;
        let token_hash = compute_bot_token_hash(token_body, &hmac_key);

        let active = bot_tokens::ActiveModel {
            id: Set(IdInstance::next_id()),
            bot_id: Set(bot_id),
            token_hash: Set(token_hash),
            token_name: Set(token_name.to_owned()),
            expires_at: Set(expires_at),
            ..Default::default()
        };

        let model = active.insert(self.get_connection()).await?;
        Ok((model, token_plain))
    }

    /// List tokens for a given bot, ordered by creation time descending.
    pub async fn list_bot_tokens(&self, bot_id: i64) -> Result<Vec<BotTokenInfo>, MegaError> {
        let models = bot_tokens::Entity::find()
            .filter(bot_tokens::Column::BotId.eq(bot_id))
            .order_by_desc(bot_tokens::Column::CreatedAt)
            .all(self.get_connection())
            .await?;

        Ok(models
            .into_iter()
            .map(|m| BotTokenInfo {
                id: m.id,
                token_name: m.token_name,
                expires_at: m.expires_at,
                revoked: m.revoked,
                created_at: m.created_at,
            })
            .collect())
    }

    /// Revoke a single token for a bot. Idempotent.
    pub async fn revoke_bot_token(
        &self,
        bot_id: i64,
        token_id: i64,
    ) -> Result<(), MegaError> {
        let conn = self.get_connection();

        if let Some(model) = bot_tokens::Entity::find_by_id(token_id)
            .filter(bot_tokens::Column::BotId.eq(bot_id))
            .one(conn)
            .await?
        {
            let mut active: bot_tokens::ActiveModel = model.into_active_model();
            active.revoked = Set(true);
            let _ = active.update(conn).await?;
        }

        Ok(())
    }

    /// Revoke all tokens belonging to the given bot. Idempotent.
    pub async fn revoke_bot_tokens_by_bot(&self, bot_id: i64) -> Result<(), MegaError> {
        let conn = self.get_connection();

        let models = bot_tokens::Entity::find()
            .filter(bot_tokens::Column::BotId.eq(bot_id))
            .all(conn)
            .await?;

        if models.is_empty() {
            return Ok(());
        }

        for model in models {
            if model.revoked {
                continue;
            }
            let mut active: bot_tokens::ActiveModel = model.into_active_model();
            active.revoked = Set(true);
            let _ = active.update(conn).await?;
        }

        Ok(())
    }

    /// Find bot and token by plaintext token string (with or without `bot_` prefix).
    pub async fn find_bot_by_token(
        &self,
        token_plain: &str,
    ) -> Result<Option<(bots::Model, bot_tokens::Model)>, MegaError> {
        let token_body = token_plain
            .strip_prefix(BOT_TOKEN_PREFIX)
            .unwrap_or(token_plain);

        let hmac_key = match load_bot_token_hmac_key() {
            Ok(key) => key,
            Err(e) => return Err(e),
        };
        let token_hash = compute_bot_token_hash(token_body, &hmac_key);

        let now = Utc::now();

        let conn = self.get_connection();

        let token = bot_tokens::Entity::find()
            .filter(bot_tokens::Column::TokenHash.eq(token_hash))
            .filter(bot_tokens::Column::Revoked.eq(false))
            .filter(
                Condition::any()
                    .add(bot_tokens::Column::ExpiresAt.is_null())
                    .add(bot_tokens::Column::ExpiresAt.gt(now)),
            )
            .one(conn)
            .await?;

        let Some(token) = token else {
            return Ok(None);
        };

        let bot = bots::Entity::find_by_id(token.bot_id)
            .one(conn)
            .await?;

        let Some(bot) = bot else {
            return Ok(None);
        };

        Ok(Some((bot, token)))
    }
}

type HmacSha256 = Hmac<Sha256>;

fn generate_bot_token_plain() -> Result<String, MegaError> {
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut bytes = [0u8; BOT_TOKEN_RANDOM_LEN];
    rng.fill(&mut bytes).map_err(|_| {
        MegaError::Other("failed to generate secure random bytes for bot token".to_string())
    })?;

    let encoded = BASE64_STANDARD.encode(bytes);
    Ok(format!("{BOT_TOKEN_PREFIX}{encoded}"))
}

fn load_bot_token_hmac_key() -> Result<Vec<u8>, MegaError> {
    let secret = std::env::var(BOT_TOKEN_HMAC_KEY_ENV).map_err(|_| {
        MegaError::Other(format!(
            "{BOT_TOKEN_HMAC_KEY_ENV} is not set for bot token HMAC"
        ))
    })?;
    Ok(secret.into_bytes())
}

fn compute_bot_token_hash(token_body: &str, key: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(key).expect("HMAC-SHA256 can take a key of any size");
    mac.update(token_body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

