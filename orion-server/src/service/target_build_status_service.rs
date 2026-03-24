use callisto::{sea_orm_active_enums::OrionTargetStatusEnum, target_build_status};
use chrono::Utc;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    sea_query::OnConflict,
};
use uuid::Uuid;

pub struct TargetBuildStatusService;

impl TargetBuildStatusService {
    #[allow(clippy::too_many_arguments)]
    pub fn new_active_model(
        id: Uuid,
        task_id: Uuid,
        target_package: String,
        target_name: String,
        target_configuration: String,
        category: String,
        identifier: String,
        action: String,
        status: OrionTargetStatusEnum,
    ) -> target_build_status::ActiveModel {
        let now = Utc::now().into();
        target_build_status::ActiveModel {
            id: Set(id),
            task_id: Set(task_id),
            target_package: Set(target_package),
            target_name: Set(target_name),
            target_configuration: Set(target_configuration),
            category: Set(category),
            identifier: Set(identifier),
            action: Set(action),
            status: Set(status),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    pub async fn upsert_batch(
        conn: &DatabaseConnection,
        models: Vec<target_build_status::ActiveModel>,
    ) -> Result<(), sea_orm::DbErr> {
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
        conn: &DatabaseConnection,
        task_id: Uuid,
    ) -> Result<Vec<target_build_status::Model>, sea_orm::DbErr> {
        target_build_status::Entity::find()
            .filter(target_build_status::Column::TaskId.eq(task_id))
            .all(conn)
            .await
    }

    pub async fn find_by_id(
        conn: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<target_build_status::Model>, sea_orm::DbErr> {
        target_build_status::Entity::find()
            .filter(target_build_status::Column::Id.eq(id))
            .one(conn)
            .await
    }
}
