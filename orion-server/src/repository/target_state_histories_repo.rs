use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter as _,
};
use uuid::Uuid;

pub struct TargetStateHistoriesRepo;

impl TargetStateHistoriesRepo {
    pub async fn upsert_state(
        conn: &impl ConnectionTrait,
        build_target_id: Uuid,
        build_event_id: Uuid,
        target_state: String,
        created_at: sea_orm::prelude::DateTimeWithTimeZone,
    ) -> Result<(), DbErr> {
        match callisto::target_state_histories::Entity::find()
            .filter(callisto::target_state_histories::Column::BuildTargetId.eq(build_target_id))
            .filter(callisto::target_state_histories::Column::BuildEventId.eq(build_event_id))
            .one(conn)
            .await?
        {
            Some(existing) => {
                let mut active: callisto::target_state_histories::ActiveModel =
                    existing.into_active_model();
                active.target_state = Set(target_state);
                let _ = active.update(conn).await?;
            }
            None => {
                let _ = callisto::target_state_histories::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    build_target_id: Set(build_target_id),
                    build_event_id: Set(build_event_id),
                    target_state: Set(target_state),
                    created_at: Set(created_at),
                }
                .insert(conn)
                .await?;
            }
        }
        Ok(())
    }
}
