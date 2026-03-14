use callisto::{
    bot_installations,
    sea_orm_active_enums::{InstallationBotStatusEnum, InstallationTargetTypeEnum},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct BotRes {
    pub id: i64,
    pub bot_id: i64,
    pub target_type: InstallationTargetType,
    pub target_id: i64,
    pub status: InstallationBotStatus,
    pub installed_by: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct InstallBotReq {
    pub target_type: InstallationTargetType,
    pub target_id: i64,
    pub installed_by: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChangeInstallationStatus {
    pub target_type: InstallationTargetType,
    pub status: InstallationBotStatus,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub enum InstallationBotStatus {
    Disabled,
    Enabled,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub enum InstallationTargetType {
    Organization,
    Repository,
}

impl From<bot_installations::Model> for BotRes {
    fn from(value: bot_installations::Model) -> Self {
        Self {
            id: value.id,
            bot_id: value.bot_id,
            target_type: value.target_type.into(),
            target_id: value.target_id,
            status: value.status.into(),
            installed_by: value.installed_by,
        }
    }
}

impl From<InstallationBotStatusEnum> for InstallationBotStatus {
    fn from(value: InstallationBotStatusEnum) -> Self {
        match value {
            InstallationBotStatusEnum::Disabled => InstallationBotStatus::Disabled,
            InstallationBotStatusEnum::Enabled => InstallationBotStatus::Enabled,
        }
    }
}

impl From<InstallationTargetTypeEnum> for InstallationTargetType {
    fn from(value: InstallationTargetTypeEnum) -> Self {
        match value {
            InstallationTargetTypeEnum::Organization => InstallationTargetType::Organization,
            InstallationTargetTypeEnum::Repository => InstallationTargetType::Repository,
        }
    }
}

impl From<InstallationBotStatus> for InstallationBotStatusEnum {
    fn from(value: InstallationBotStatus) -> Self {
        match value {
            InstallationBotStatus::Disabled => InstallationBotStatusEnum::Disabled,
            InstallationBotStatus::Enabled => InstallationBotStatusEnum::Enabled,
        }
    }
}

impl From<InstallationTargetType> for InstallationTargetTypeEnum {
    fn from(value: InstallationTargetType) -> Self {
        match value {
            InstallationTargetType::Organization => InstallationTargetTypeEnum::Organization,
            InstallationTargetType::Repository => InstallationTargetTypeEnum::Repository,
        }
    }
}
