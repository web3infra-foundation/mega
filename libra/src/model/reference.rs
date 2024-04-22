use sea_orm::entity::prelude::*;
use sea_orm::QueryFilter;
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

// some useful functions
impl Model {
    pub async fn current_head(db: &DbConn) -> Result<Option<Self>, DbErr> {
        Ok(self::Entity::find()
            .filter(self::Column::Kind.eq(self::ConfigKind::Head))
            .one(db)
            .await
            .unwrap())
    }

    pub async fn find_branch_by_name(db: &DbConn, name: &str) -> Result<Option<Self>, DbErr> {
        Ok(self::Entity::find()
            .filter(self::Column::Name.eq(name))
            .filter(self::Column::Kind.eq(self::ConfigKind::Branch))
            .one(db)
            .await
            .unwrap())
    }
}
