use sea_orm::QuerySelect;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Database model for build tasks
/// Stores information about build jobs including their status, timing, and metadata
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub task_id: Uuid,
    #[sea_orm(column_type = "JsonBinary")]
    pub build_ids: Json,
    #[sea_orm(column_type = "JsonBinary")]
    pub output_files: Json,
    pub exit_code: Option<i32>,
    pub start_at: DateTimeWithTimeZone,
    pub end_at: Option<DateTimeWithTimeZone>,
    pub repo_name: String,
    pub target: String,
    pub arguments: String,
    pub mr: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Retrieves a task record by its UUID from the database
    pub async fn get_by_task_id(task_id: Uuid, conn: &DatabaseConnection) -> Option<Model> {
        Entity::find()
            .filter(Column::TaskId.eq(task_id))
            .one(conn)
            .await
            .expect("Failed to get by `task_id`")
    }

    /// Retrieves build_ids list by task_id
    pub async fn get_builds_by_task_id(
        task_id: Uuid,
        conn: &DatabaseConnection,
    ) -> Option<Vec<Uuid>> {
        let build_ids: Option<serde_json::Value> = Entity::find()
            .filter(Column::TaskId.eq(task_id))
            .select_only()
            .column(Column::BuildIds)
            .into_tuple::<serde_json::Value>()
            .one(conn)
            .await
            .expect("Failed to get `build_ids` by `task_id`");

        build_ids
            .map(|json| serde_json::from_value::<Vec<Uuid>>(json).unwrap_or_else(|_| Vec::new()))
    }

    /// Checks if a task with the given task_id exists in the database
    pub async fn exists_by_task_id(task_id: Uuid, conn: &DatabaseConnection) -> bool {
        Entity::find()
            .filter(Column::TaskId.eq(task_id))
            .count(conn)
            .await
            .expect("Failed to check if task exists")
            > 0
    }
}
