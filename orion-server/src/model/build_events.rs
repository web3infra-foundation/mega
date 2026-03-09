use chrono::Utc;
use sea_orm::{ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize)]
pub struct BuildEventDTO {
    pub id: String,
    pub task_id: String,
    pub retry_count: i32,
    pub exit_code: Option<i32>,
    pub log: Option<String>,
    pub log_output_file: String,
    pub start_at: String,
    pub end_at: Option<String>,
}

impl From<&callisto::build_events::Model> for BuildEventDTO {
    fn from(model: &callisto::build_events::Model) -> Self {
        Self {
            id: model.id.to_string(),
            task_id: model.task_id.to_string(),
            retry_count: model.retry_count,
            exit_code: model.exit_code,
            log: model.log.clone(),
            log_output_file: model.log_output_file.clone(),
            start_at: model.start_at.to_string(),
            end_at: model.end_at.map(|dt| dt.with_timezone(&Utc).to_string()),
        }
    }
}

pub struct BuildEvent;

impl BuildEvent {
    pub async fn update_build_complete_result(
        build_id: &str,
        exit_code: Option<i32>,
        _success: bool,
        _message: &str,
        db_connection: &impl ConnectionTrait,
    ) -> Result<(), DbErr> {
        callisto::build_events::Entity::update_many()
            .filter(callisto::build_events::Column::Id.eq(build_id.parse::<Uuid>().unwrap()))
            .set(callisto::build_events::ActiveModel {
                exit_code: Set(exit_code),
                ..Default::default()
            })
            .exec(db_connection)
            .await?;
        Ok(())
    }
}
