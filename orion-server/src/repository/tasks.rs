use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait,
    QueryFilter,
};
use uuid::Uuid;

pub struct TaskRepository;

impl TaskRepository {
    /// Retrieves build IDs associated with a task ID
    pub async fn get_builds_by_task_id(
        task_id: Uuid,
        db: &sea_orm::DatabaseConnection,
    ) -> Option<Vec<Uuid>> {
        match crate::entity::builds::Entity::find()
            .filter(crate::entity::builds::Column::TaskId.eq(task_id))
            .all(db)
            .await
        {
            Ok(builds) => Some(builds.into_iter().map(|build| build.id).collect()),
            Err(e) => {
                tracing::error!("Failed to fetch builds for task_id {}: {}", task_id, e);
                None
            }
        }
    }

    /// Create a new task ActiveModel for database insertion
    pub fn create_task(
        task_id: Uuid,
        cl_id: i64,
        task_name: Option<String>,
        template: Option<sea_orm::prelude::Json>,
        created_at: sea_orm::prelude::DateTimeWithTimeZone,
    ) -> crate::entity::tasks::ActiveModel {
        crate::entity::tasks::ActiveModel {
            id: Set(task_id),
            cl_id: Set(cl_id),
            task_name: Set(task_name),
            template: Set(template),
            created_at: Set(created_at),
        }
    }

    /// Insert a task directly into the database
    pub async fn insert_task(
        task_id: Uuid,
        cl_id: i64,
        task_name: Option<String>,
        template: Option<sea_orm::prelude::Json>,
        created_at: sea_orm::prelude::DateTimeWithTimeZone,
        db: &impl ConnectionTrait,
    ) -> Result<crate::entity::tasks::Model, DbErr> {
        let task_model = Self::create_task(task_id, cl_id, task_name, template, created_at);
        task_model.insert(db).await
    }
}
