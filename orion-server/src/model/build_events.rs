use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;

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
