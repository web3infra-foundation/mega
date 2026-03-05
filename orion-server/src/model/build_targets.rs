use core::task;

use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel, QueryFilter,
};
use uuid::Uuid;

use crate::model::targets::TargetState;
/// A collection of utility methods for the `build_targets` database table.
pub struct BuildTarget;

impl BuildTarget {
    pub fn create_default_target(id: Uuid, task_id: Uuid) -> callisto::build_targets::Model {
        let default_path = "//";
        callisto::build_targets::Model {
            id,
            task_id,
            path: default_path.to_string(),
            latest_state: "NOT_STARTED".to_string(),
        }
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

    pub async fn find_build_targets(
        build_id: Uuid,
        task_id: Uuid,
        db: &impl ConnectionTrait,
    ) -> Result<Vec<BuildTargetDTO>, DbErr> {
        // Get all targets of corresponding build
        let all_targets = callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .all(db)
            .await?;

        let mut result = Vec::with_capacity(all_targets.len());

        // Search corresponding target state under current build
        for target in all_targets {
            let status = callisto::target_build_status::Entity::find()
                .filter(callisto::target_build_status::Column::TargetId.eq(target.id))
                .filter(callisto::target_build_status::Column::BuildId.eq(build_id))
                .one(db)
                .await?;

            let state = match status {
                Some(s) => TargetState::from(s.state.as_str()),
                None => TargetState::NotStarted,
            };

            result.push(BuildTargetDTO {
                id: target.id,
                path: target.path,
                state,
            });
        }

        Ok(result)
    }

    pub async fn update_build_targets(
        target_state: TargetState,
        build_id: Uuid,
        db: &impl ConnectionTrait,
    ) -> Result<(), DbErr> {
        let all_targets = callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .all(db)
            .await?;

        for target in all_targets {
            let status = callisto::target_build_status::Entity::find()
                .filter(callisto::target_build_status::Column::TargetId.eq(target.id))
                .filter(callisto::target_build_status::Column::BuildId.eq(build_id))
                .one(db)
                .await?;

            if let Some(s) = status {
                let mut active: callisto::target_build_status::ActiveModel = s.into_active_model();
                active.state = sea_orm::Set(target_state.to_string());
                active.update(db).await?;
            }
        }
    }
}

pub struct BuildTargetDTO {
    id: Uuid,
    path: String,
    state: TargetState,
}
