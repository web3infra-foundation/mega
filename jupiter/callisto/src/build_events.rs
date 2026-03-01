use chrono::Utc;
/// User-defined build event, should be changed later in migration to fit
use sea_orm::{ActiveValue::Set, entity::prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "build_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub task_id: Uuid,
    pub retry_count: i32,
    pub exit_code: Option<i32>,
    pub log: Option<String>,
    pub log_output_file: String,
    pub start_at: DateTimeWithTimeZone,
    pub end_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::orion_tasks::Entity",
        from = "Column::TaskId",
        to = "super::orion_tasks::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    OrionTasks,
}

impl Related<super::orion_tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrionTasks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a new build ActiveModel for database insertion
    pub fn create_build(build_id: Uuid, task_id: Uuid, repo: String) -> ActiveModel {
        let now = Utc::now().into();
        let repo_leaf = repo
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&repo)
            .to_string();
        ActiveModel {
            id: Set(build_id),
            task_id: Set(task_id),
            exit_code: Set(None),
            start_at: Set(now),
            end_at: Set(None),
            retry_count: Set(0),
            log: Set(None),
            // TODO: set correct log output file
            log_output_file: Set(format!("{}/{}/{}.log", task_id, repo_leaf, build_id)),
        }
    }

    /// Insert a single build directly into the database
    pub async fn insert_build(
        build_id: Uuid,
        task_id: Uuid,
        repo: String,
        db: &impl ConnectionTrait,
    ) -> Result<Model, DbErr> {
        let build_model = Self::create_build(build_id, task_id, repo);
        build_model.insert(db).await
    }
}
