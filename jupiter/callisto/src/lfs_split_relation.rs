use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "lfs_split_relations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub ori_oid: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub sub_oid: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub offset: i64,
    pub size: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}