use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "reference")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub reference_id: i64,
    pub name: String,
    pub type_: String, // type is a reserved keyword
    pub commit_hash: Option<String>,
    pub scope: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
