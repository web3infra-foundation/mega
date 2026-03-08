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
        // Register and insert a new bot
        let bot = self
            .new_bot_model(
                name,
                organization_id,
                creator_user_id,
                permission_scope,
                BotStatusEnum::Enabled,
            )
            .await?;

        // 2. 生成 RSA 密钥对
        let mut rng = OsRng;
        let private_key =
            RsaPrivateKey::new(&mut rng, 2048).map_err(|e| MegaError::Other(e.to_string()))?;
        let public_key = private_key.to_public_key();

        // 转 PEM 格式
        let private_pem = private_key
            .to_pkcs8_pem(Default::default())
            .map_err(|e| MegaError::Other(e.to_string()))?
            .to_string();
        let public_pem = public_key
            .to_public_key_pem(Default::default())
            .map_err(|e| MegaError::Other(e.to_string()))?
            .to_string();

        // 3. 保存 BotKeys
        // bot_keys::ActiveModel {
        //     bot_id: Set(bot.id),
        //     private_key: Set(private_pem.clone()),
        //     public_key: Set(public_pem),
        //     ..Default::default()
        // }
        // .insert(self.get_connection())
        // .await?;

        // 4. 可选：初始化 token / permissions / audit log 等，方便未来扩展
        // 例如：
        // self.init_bot_permissions(bot.id, &mut tx).await?;

        // 6. 返回 Bot 记录和 private key（只返回一次）
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

    pub async fn new_bot_key() {}
}
