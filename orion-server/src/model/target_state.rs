use std::fmt::Display;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, Copy, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum TargetState {
    #[sea_orm(string_value = "Pending")]
    Pending,
    #[sea_orm(string_value = "Building")]
    Building,
    #[sea_orm(string_value = "Completed")]
    Completed,
    #[sea_orm(string_value = "Failed")]
    Failed,
    #[sea_orm(string_value = "Interrupted")]
    Interrupted,
    #[sea_orm(string_value = "Uninitialized")]
    Uninitialized,
}

impl From<String> for TargetState {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Pending" => TargetState::Pending,
            "Building" => TargetState::Building,
            "Completed" => TargetState::Completed,
            "Failed" => TargetState::Failed,
            "Interrupted" => TargetState::Interrupted,
            "Uninitialized" => TargetState::Uninitialized,
            _ => TargetState::Pending,
        }
    }
}

impl Display for TargetState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TargetState::Pending => "Pending",
            TargetState::Building => "Building",
            TargetState::Completed => "Completed",
            TargetState::Failed => "Failed",
            TargetState::Interrupted => "Interrupted",
            TargetState::Uninitialized => "Uninitialized",
        };
        write!(f, "{}", s)
    }
}
