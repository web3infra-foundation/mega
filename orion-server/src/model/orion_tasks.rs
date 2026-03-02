use serde_json::{Value, to_value};

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

#[derive(ToSchema, Serialize)]
pub struct OrionTaskDTO {
    pub id: String,
    pub changes: Value,
    pub repo_name: String,
    pub cl: String,
    pub created_at: String,
}

impl From<&callisto::orion_tasks::Model> for OrionTaskDTO {
    fn from(model: &callisto::orion_tasks::Model) -> Self {
        Self {
            id: model.id.to_string(),
            changes: model.changes.clone(),
            repo_name: model.repo_name.clone(),
            cl: model.cl.clone(),
            created_at: model.created_at.with_timezone(&Utc).to_string(),
        }
    }
}
