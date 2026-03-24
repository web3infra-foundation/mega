use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbErr};
use serde_json::Value;
use uuid::Uuid;

pub struct BuildRepository;

impl BuildRepository {
    /// Create a new build ActiveModel for database insertion
    pub fn create_build(
        build_id: Uuid,
        task_id: Uuid,
        target_id: Uuid,
        repo: String,
        args: Option<Value>,
    ) -> crate::entity::builds::ActiveModel {
        let now = Utc::now().into();
        let repo_leaf = repo
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&repo)
            .to_string();
        crate::entity::builds::ActiveModel {
            id: Set(build_id),
            task_id: Set(task_id),
            target_id: Set(target_id),
            exit_code: Set(None),
            start_at: Set(now),
            end_at: Set(None),
            repo: Set(repo),
            args: Set(args),
            output_file: Set(format!("{}/{}/{}.log", task_id, repo_leaf, build_id)),
            created_at: Set(now),
            retry_count: Set(0),
        }
    }

    /// Insert a single build directly into the database
    pub async fn insert_build(
        build_id: Uuid,
        task_id: Uuid,
        target_id: Uuid,
        repo: String,
        db: &impl ConnectionTrait,
    ) -> Result<crate::entity::builds::Model, DbErr> {
        let build_model = Self::create_build(build_id, task_id, target_id, repo, None);
        build_model.insert(db).await
    }
}
