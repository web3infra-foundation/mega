use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter as _,
};
use uuid::Uuid;

use crate::model::{internal::BuildTargetStateDTO, target_state::TargetState};

pub struct BuildTargetsRepo;

type BuildTargetDTO = BuildTargetStateDTO;

impl BuildTargetsRepo {
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

    pub(crate) async fn find_initialized_build_targets(
        build_id: Uuid,
        task_id: Uuid,
        db: &impl ConnectionTrait,
    ) -> Result<Vec<BuildTargetDTO>, DbErr> {
        if Self::has_uninitialized_target(task_id, db).await? {
            return Ok(vec![]);
        }

        let all_targets = callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .filter(
                callisto::build_targets::Column::LatestState
                    .ne(TargetState::Uninitialized.to_string()),
            )
            .all(db)
            .await?;

        let mut result = Vec::with_capacity(all_targets.len());

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

            result.push(BuildTargetStateDTO {
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

    pub async fn find_by_id(
        conn: &impl ConnectionTrait,
        id: Uuid,
    ) -> Result<Option<callisto::build_targets::Model>, DbErr> {
        callisto::build_targets::Entity::find_by_id(id)
            .one(conn)
            .await
    }

    pub async fn list_by_task_id(
        conn: &impl ConnectionTrait,
        task_id: Uuid,
    ) -> Result<Vec<callisto::build_targets::Model>, DbErr> {
        callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .all(conn)
            .await
    }

    pub async fn ensure_any_target_for_task(
        conn: &impl ConnectionTrait,
        task_id: Uuid,
    ) -> Result<callisto::build_targets::Model, DbErr> {
        if let Some(t) = callisto::build_targets::Entity::find()
            .filter(callisto::build_targets::Column::TaskId.eq(task_id))
            .one(conn)
            .await?
        {
            return Ok(t);
        }

        let id = Uuid::now_v7();
        let _ = Self::insert_default_target(id, task_id, conn).await?;
        Ok(callisto::build_targets::Entity::find_by_id(id)
            .one(conn)
            .await?
            .unwrap_or(callisto::build_targets::Model {
                id,
                task_id,
                path: "//".to_string(),
                latest_state: TargetState::Uninitialized.to_string(),
            }))
    }

    pub async fn update_latest_state(
        conn: &impl ConnectionTrait,
        build_target_id: Uuid,
        state: TargetState,
    ) -> Result<(), DbErr> {
        callisto::build_targets::Entity::update_many()
            .filter(callisto::build_targets::Column::Id.eq(build_target_id))
            .set(callisto::build_targets::ActiveModel {
                latest_state: sea_orm::Set(state.to_string()),
                ..Default::default()
            })
            .exec(conn)
            .await?;
        Ok(())
    }
}
