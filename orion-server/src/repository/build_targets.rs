use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter,
};
use uuid::Uuid;

use crate::entity::targets::TargetState;

/// A collection of utility methods for the `build_targets` database table.
pub struct BuildTarget;

impl BuildTarget {
    pub fn create_default_target(id: Uuid, task_id: Uuid) -> callisto::build_targets::Model {
        let default_path = "//";
        callisto::build_targets::Model {
            id,
            task_id,
            path: default_path.to_string(),
            latest_state: TargetState::Uninitialized.to_string(),
        }
    }

    /// Check if there is any target with `Uninitialized` state for the given task_id.
    pub async fn has_uninitialized_target(
        task_id: Uuid,
        db: &impl ConnectionTrait,
    ) -> Result<bool, DbErr> {
        let target = callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .filter(
                callisto::build_targets::Column::LatestState
                    .eq(TargetState::Uninitialized.to_string()),
            )
            .one(db)
            .await?;
        Ok(target.is_some())
    }

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

    #[allow(dead_code)]
    pub async fn find_initialized_build_targets(
        build_id: Uuid,
        task_id: Uuid,
        db: &impl ConnectionTrait,
    ) -> Result<Vec<BuildTargetDTO>, DbErr> {
        if Self::has_uninitialized_target(task_id, db).await? {
            return Ok(vec![]);
        }

        // Get all targets of corresponding build
        let all_targets = callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .filter(
                callisto::build_targets::Column::LatestState
                    .ne(TargetState::Uninitialized.to_string()),
            )
            .all(db)
            .await?;

        let mut result = Vec::with_capacity(all_targets.len());

        // Search corresponding target state under current build
        for target in all_targets {
            let status = callisto::target_state_histories::Entity::find()
                .filter(callisto::target_state_histories::Column::BuildTargetId.eq(target.id))
                .filter(callisto::target_state_histories::Column::BuildEventId.eq(build_id))
                .one(db)
                .await?;

            let state = match status {
                Some(s) => TargetState::from(s.target_state),
                None => TargetState::Uninitialized,
            };

            result.push(BuildTargetDTO {
                id: target.id,
                path: target.path,
                state,
            });
        }

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn update_build_targets(
        target_state: TargetState,
        build_id: &str,
        db: &impl ConnectionTrait,
    ) -> Result<(), DbErr> {
        let build_id = Uuid::parse_str(build_id)
            .map_err(|e| DbErr::Custom(format!("Invalid build_id format: {}", e)))?;
        let task_id = callisto::build_events::Entity::find_by_id(build_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                DbErr::RecordNotFound(format!("Build event with id {} not found", build_id))
            })?
            .task_id;

        let all_targets = callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .all(db)
            .await?;

        for target in all_targets {
            let status = callisto::target_state_histories::Entity::find()
                .filter(callisto::target_state_histories::Column::BuildTargetId.eq(target.id))
                .filter(callisto::target_state_histories::Column::BuildEventId.eq(build_id))
                .one(db)
                .await?;

            if let Some(s) = status {
                let mut active: callisto::target_state_histories::ActiveModel =
                    s.into_active_model();
                active.target_state = sea_orm::Set(target_state.to_string());
                active.update(db).await?;
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BuildTargetDTO {
    id: Uuid,
    path: String,
    state: TargetState,
}
