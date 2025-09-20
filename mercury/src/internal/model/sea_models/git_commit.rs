use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "git_commit")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub repo_id: i32,
    #[sea_orm(unique)]
    pub commit_id: String,
    pub tree: String,
    #[sea_orm(column_type = "Text")]
    pub parents_id: String,
    pub author: Option<String>,
    pub committer: Option<String>,
    pub content: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

