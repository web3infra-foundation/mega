use callisto::target_build_status;
use chrono::Utc;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter as _,
    QuerySelect as _, sea_query::OnConflict,
};
use uuid::Uuid;

use crate::model::internal::target_build_status::NewTargetStatusInput;

pub struct TargetBuildStatusRepo;

impl TargetBuildStatusRepo {
    pub(crate) fn new_active_model(
        input: NewTargetStatusInput,
    ) -> target_build_status::ActiveModel {
        let now = Utc::now().into();
        target_build_status::ActiveModel {
            id: Set(input.id),
            task_id: Set(input.task_id),
            target_package: Set(input.target_package),
            target_name: Set(input.target_name),
            target_configuration: Set(input.target_configuration),
            category: Set(input.category),
            identifier: Set(input.identifier),
            action: Set(input.action),
            status: Set(input.status),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    pub async fn upsert_batch(
        conn: &impl ConnectionTrait,
        models: Vec<target_build_status::ActiveModel>,
    ) -> Result<(), DbErr> {
        if models.is_empty() {
            return Ok(());
        }

        target_build_status::Entity::insert_many(models)
            .on_conflict(
                OnConflict::columns([
                    target_build_status::Column::TaskId,
                    target_build_status::Column::TargetPackage,
                    target_build_status::Column::TargetName,
                    target_build_status::Column::TargetConfiguration,
                    target_build_status::Column::Category,
                    target_build_status::Column::Identifier,
                    target_build_status::Column::Action,
                ])
                .update_columns([
                    target_build_status::Column::Status,
                    target_build_status::Column::UpdatedAt,
                ])
                .to_owned(),
            )
            .exec(conn)
            .await?;

        Ok(())
    }

    pub async fn fetch_by_task_id(
        conn: &impl ConnectionTrait,
        task_id: Uuid,
    ) -> Result<Vec<target_build_status::Model>, DbErr> {
        target_build_status::Entity::find()
            .filter(target_build_status::Column::TaskId.eq(task_id))
            .all(conn)
            .await
    }

    pub async fn find_by_id(
        conn: &impl ConnectionTrait,
        id: Uuid,
    ) -> Result<Option<target_build_status::Model>, DbErr> {
        target_build_status::Entity::find_by_id(id).one(conn).await
    }

    pub async fn ping(conn: &impl ConnectionTrait) -> Result<(), DbErr> {
        let _ = target_build_status::Entity::find()
            .limit(1)
            .all(conn)
            .await?;
        Ok(())
    }
}
