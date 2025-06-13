use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "reference")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub name: Option<String>,
    pub kind: ConfigKind, // type is a reserved keyword
    pub commit: Option<String>,
    pub remote: Option<String>, // None for local, Some for remote, '' is not valid
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
/// kind enum
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "config_kind")]
pub enum ConfigKind {
    #[sea_orm(string_value = "Branch")]
    Branch, // .git/refs/heads
    #[sea_orm(string_value = "Tag")]
    Tag, // .git/refs/tags
    #[sea_orm(string_value = "Head")]
    Head, // .git/HEAD
}
