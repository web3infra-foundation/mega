use sea_orm::entity::prelude::*;
use serde::Serialize;

/// Database model for build tasks
/// Stores information about build jobs including their status, timing, and metadata
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Default)]
#[sea_orm(table_name = "builds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub build_id: Uuid,
    pub exit_code: Option<i32>,
    pub start_at: DateTime,
    pub end_at: Option<DateTime>,
    pub repo_name: String,
    pub target: String,
    #[sea_orm(column_type = "Text")]
    pub output_file: String,
    #[sea_orm(column_type = "Text")]
    pub arguments: String,
    #[sea_orm(column_type = "Text")]
    pub mr: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Retrieves a build record by its UUID from the database
    pub async fn get_by_build_id(build_id: Uuid, conn: &DatabaseConnection) -> Option<Model> {
        Entity::find()
            .filter(Column::BuildId.eq(build_id))
            .one(conn)
            .await
            .expect("Failed to get by `build_id`")
    }
}
