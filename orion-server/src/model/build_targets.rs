use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, IntoActiveModel};
use uuid::Uuid;

#[allow(dead_code)]
/// A collection of utility methods for the `build_targets` database table.
pub struct BuildTarget;

impl BuildTarget {
    #[allow(dead_code)]
    pub fn create_default_target(id: Uuid, task_id: Uuid) -> callisto::build_targets::Model {
        let default_path = "//";
        callisto::build_targets::Model {
            id,
            task_id,
            path: default_path.to_string(),
            latest_state: "NOT_STARTED".to_string(),
        }
    }

    #[allow(dead_code)]
    pub async fn insert_default_target(
        id: Uuid,
        task_id: Uuid,
        db: &impl ConnectionTrait,
    ) -> Result<String, DbErr> {
        let target = Self::create_default_target(id, task_id);
        let path = target.path.clone();
        target.into_active_model().insert(db).await?;
        Ok(path.to_string())
    }
}
