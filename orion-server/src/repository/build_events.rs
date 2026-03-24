use sea_orm::{ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct BuildEvent;

impl BuildEvent {
    pub async fn update_build_complete_result(
        build_id: &str,
        exit_code: Option<i32>,
        db_connection: &impl ConnectionTrait,
    ) -> Result<(), DbErr> {
        callisto::build_events::Entity::update_many()
            .filter(callisto::build_events::Column::Id.eq(build_id.parse::<Uuid>().unwrap()))
            .set(callisto::build_events::ActiveModel {
                exit_code: Set(exit_code),
                ..Default::default()
            })
            .exec(db_connection)
            .await?;
        Ok(())
    }
}
