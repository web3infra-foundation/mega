use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "gpg_key")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub key_id: i64,
    pub user_id: i64,
    #[sea_orm(column_type = "Text")]
    pub public_key: String,
    #[sea_orm(column_type = "Text", unique)]
    pub fingerprint: String,
    #[sea_orm(column_type = "Text")]
    pub alias: String,
    pub is_verified: bool,
    pub created_at: DateTime,
    pub expires_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}
