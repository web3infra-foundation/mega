use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait,
    QueryFilter, RuntimeErr, sqlx,
};
use uuid::Uuid;

use crate::entity::targets::{self, TargetState};

pub struct TargetRepository;

impl TargetRepository {
    /// Create a new target ActiveModel ready for insertion
    pub fn create_target(task_id: Uuid, target_path: impl Into<String>) -> targets::ActiveModel {
        let now = Utc::now().into();
        targets::ActiveModel {
            id: Set(Uuid::now_v7()),
            task_id: Set(task_id),
            target_path: Set(target_path.into()),
            state: Set(TargetState::Pending),
            start_at: Set(None),
            end_at: Set(None),
            error_summary: Set(None),
            created_at: Set(now),
        }
    }

    /// Insert a target directly into the database
    pub async fn insert_target(
        task_id: Uuid,
        target_path: impl Into<String>,
        db: &impl ConnectionTrait,
    ) -> Result<targets::Model, DbErr> {
        let target_model = Self::create_target(task_id, target_path);
        target_model.insert(db).await
    }

    /// Find an existing target for the same task and path, or create a new one.
    pub async fn find_or_create(
        db: &impl ConnectionTrait,
        task_id: Uuid,
        target_path: impl Into<String> + Clone,
    ) -> Result<targets::Model, DbErr> {
        let target_path_owned: String = target_path.clone().into();
        if let Some(target) = targets::Entity::find()
            .filter(targets::Column::TaskId.eq(task_id))
            .filter(targets::Column::TargetPath.eq(target_path_owned.clone()))
            .one(db)
            .await?
        {
            return Ok(target);
        }

        match Self::insert_target(task_id, target_path_owned.clone(), db).await {
            Ok(model) => Ok(model),
            Err(DbErr::Exec(RuntimeErr::SqlxError(sqlx::Error::Database(db_err)))) => {
                if let Some(code) = db_err.code()
                    && code == "23505"
                {
                    return targets::Entity::find()
                        .filter(targets::Column::TaskId.eq(task_id))
                        .filter(targets::Column::TargetPath.eq(target_path_owned))
                        .one(db)
                        .await?
                        .ok_or_else(|| DbErr::RecordNotFound("target".into()));
                }
                Err(DbErr::Exec(RuntimeErr::SqlxError(sqlx::Error::Database(
                    db_err,
                ))))
            }
            Err(e) => Err(e),
        }
    }

    /// Update target state and optional timing/error metadata.
    pub async fn update_state(
        db: &impl ConnectionTrait,
        target_id: Uuid,
        state: TargetState,
        start_at: Option<sea_orm::prelude::DateTimeWithTimeZone>,
        end_at: Option<sea_orm::prelude::DateTimeWithTimeZone>,
        error_summary: Option<String>,
    ) -> Result<(), DbErr> {
        let mut active = targets::ActiveModel {
            id: Set(target_id),
            state: Set(state),
            ..Default::default()
        };

        match state {
            TargetState::Building | TargetState::Pending => {
                active.end_at = Set(None);
                active.error_summary = Set(None);
            }
            _ => {}
        }

        if let Some(start_at) = start_at {
            active.start_at = Set(Some(start_at));
        }
        if let Some(end_at) = end_at {
            active.end_at = Set(Some(end_at));
        }
        if let Some(summary) = error_summary {
            active.error_summary = Set(Some(summary));
        }

        active.update(db).await.map(|_| ())
    }
}
