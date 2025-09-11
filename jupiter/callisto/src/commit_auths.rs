//! `SeaORM` Entity for commit bindings

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "commit_auths")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub commit_sha: String,
    #[sea_orm(column_type = "Text")]
    pub author_email: String,
    // matched_username links to user.username (text) stored as string to be flexible
    #[sea_orm(column_type = "Text", nullable)]
    pub matched_username: Option<String>,
    pub is_anonymous: bool,
    pub matched_at: Option<DateTime>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
