use api_model::buck2::{status::Status, types::ProjectRelativePath};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, IntoActiveModel};
use serde_json::to_value;
use uuid::Uuid;

pub struct OrionTask;

impl OrionTask {
    fn create_task(
        task_id: Uuid,
        cl_link: &str,
        repo: &str,
        changes: &Vec<Status<ProjectRelativePath>>,
    ) -> Result<callisto::orion_tasks::Model, serde_json::Error> {
        Ok(callisto::orion_tasks::Model {
            id: task_id,
            cl: cl_link.to_string(),
            repo_name: repo.to_string(),
            changes: to_value(changes)?,
            created_at: chrono::Utc::now().into(),
        })
    }

    pub async fn insert_task(
        task_id: Uuid,
        cl_link: &str,
        repo: &str,
        changes: &Vec<Status<ProjectRelativePath>>,
        db: &impl ConnectionTrait,
    ) -> Result<callisto::orion_tasks::Model, DbErr> {
        let task_model = Self::create_task(task_id, cl_link, repo, changes)
            .map_err(|e| DbErr::Custom(e.to_string()))?;
        task_model.into_active_model().insert(db).await
    }
}
