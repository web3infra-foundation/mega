use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "builds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub build_id: Uuid,
    pub output: String,
    pub exit_code: Option<i32>, // On Unix, return `None` if the process was terminated by a signal.
    pub start_at: DateTimeUtc,
    pub end_at: DateTimeUtc,
    pub repo_name: String,
    pub target: String, // build target, e.g. "//:main"
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}