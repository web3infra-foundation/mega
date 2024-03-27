use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "config_section")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub section_id: i32,
    pub section_name: String,
    pub unique_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
