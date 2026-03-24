use std::fmt::Display;

use chrono::Utc;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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
            _ => TargetState::Pending, // Default to Pending for unknown states
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

/// Target DTO with a generic builds payload.
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct TargetWithBuilds<T> {
    pub id: String,
    pub target_path: String,
    pub state: TargetState,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub error_summary: Option<String>,
    pub builds: Vec<T>,
}

impl<T> TargetWithBuilds<T> {
    pub fn from_model(model: Model, builds: Vec<T>) -> Self {
        Self {
            id: model.id.to_string(),
            target_path: model.target_path,
            state: model.state,
            start_at: model.start_at.map(|dt| dt.with_timezone(&Utc).to_rfc3339()),
            end_at: model.end_at.map(|dt| dt.with_timezone(&Utc).to_rfc3339()),
            error_summary: model.error_summary,
            builds,
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "targets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub task_id: Uuid,
    pub target_path: String,
    pub state: TargetState,
    pub start_at: Option<DateTimeWithTimeZone>,
    pub end_at: Option<DateTimeWithTimeZone>,
    pub error_summary: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tasks::Entity",
        from = "Column::TaskId",
        to = "super::tasks::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Tasks,
    #[sea_orm(has_many = "super::builds::Entity")]
    Builds,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tasks.def()
    }
}

impl Related<super::builds::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Builds.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
