use callisto::sea_orm_active_enums::{
    InstallationBotStatusEnum, PermissionEnum, PermissionScopeEnum,
};
use common::errors::MegaError;

use crate::api_service::mono_api_service::MonoApiService;

impl MonoApiService {
    /// Check whether a bot has sufficient permission on a given resource.
    ///
    /// The decision is based on:
    /// - Bot status (must be enabled).
    /// - Whether the bot has at least one enabled installation record.
    /// - The bot-level `permission_scope` compared against `required_permission`.
    ///
    /// Installation scope is currently checked in an aggregated way (any enabled installation
    /// is sufficient) and can be refined later to be resource-type aware.
    pub async fn check_bot_permission(
        &self,
        bot_id: i64,
        _resource_type: callisto::sea_orm_active_enums::ResourceTypeEnum,
        _resource_id: &str,
        required_permission: PermissionEnum,
    ) -> Result<bool, MegaError> {
        let bots_storage = self.storage.bots_storage();

        // 1. Load bot and ensure it is enabled.
        let bot = match bots_storage.get_bot_by_id(bot_id).await? {
            Some(b) => b,
            None => return Ok(false),
        };

        if bot.status != callisto::sea_orm_active_enums::BotStatusEnum::Enabled {
            return Ok(false);
        }

        // 2. Ensure the bot has at least one enabled installation.
        let installations = bots_storage.get_installed_bot_by_id(bot_id).await?;
        let has_enabled_installation = installations
            .iter()
            .any(|inst| inst.status == InstallationBotStatusEnum::Enabled);

        if !has_enabled_installation {
            return Ok(false);
        }

        // 3. Compare bot-level permission scope with the required permission.
        Ok(scope_satisfies_permission(
            &bot.permission_scope,
            &required_permission,
        ))
    }
}

fn scope_level(scope: &PermissionScopeEnum) -> u8 {
    match scope {
        PermissionScopeEnum::Read => 1,
        PermissionScopeEnum::Write => 2,
        PermissionScopeEnum::Admin => 3,
    }
}

fn permission_level(permission: &PermissionEnum) -> u8 {
    match permission {
        PermissionEnum::Read => 1,
        PermissionEnum::Write => 2,
        PermissionEnum::Admin => 3,
    }
}

fn scope_satisfies_permission(scope: &PermissionScopeEnum, required: &PermissionEnum) -> bool {
    scope_level(scope) >= permission_level(required)
}
