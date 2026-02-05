/// User-defined build event, should be changed later in migration to fit
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "build_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub task_id: Uuid,
    pub exit_code: Option<i32>,
    pub retry_count: i32,
    pub repo: String,
    pub log: Option<String>,
    pub log_output_file: String,
    pub start_at: DateTimeWithTimeZone,
    pub end_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tasks::Entity",
        from = "Column::TaskId",
        to = "super::tasks::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Tasks,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tasks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

trait BuildModel {
    /// Crete a new build event ActiveModel for database insertion
    fn create_build_event(
        build_event_id: Uuid,
        task_id: Uuid,
        repo: String,
        retry_count: i32,
    ) -> ActiveModel;

    fn insert_build_event(
        build_event_id: Uuid,
        task_id: Uuid,
        repo: String,
        retry_count: i32,
        db: &impl ConnectionTrait,
    ) -> Result<Model, DbErr>;
}
