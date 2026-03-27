use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait,
    QueryFilter as _, QueryOrder,
};
use uuid::Uuid;

pub struct BuildEventsRepo;

impl BuildEventsRepo {
    fn create_build_model(
        build_id: Uuid,
        task_id: Uuid,
        repo: String,
    ) -> callisto::build_events::ActiveModel {
        let now = Utc::now().into();
        let repo_leaf = repo
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&repo)
            .to_string();

        callisto::build_events::ActiveModel {
            id: Set(build_id),
            task_id: Set(task_id),
            exit_code: Set(None),
            start_at: Set(now),
            end_at: Set(None),
            retry_count: Set(0),
            log: Set(None),
            // TODO: set correct log output file
            log_output_file: Set(format!("{}/{}/{}.log", task_id, repo_leaf, build_id)),
        }
    }

    pub async fn find_by_id(
        conn: &impl ConnectionTrait,
        id: Uuid,
    ) -> Result<Option<callisto::build_events::Model>, DbErr> {
        callisto::build_events::Entity::find_by_id(id)
            .one(conn)
            .await
    }

    pub async fn list_by_task_id(
        conn: &impl ConnectionTrait,
        task_id: Uuid,
    ) -> Result<Vec<callisto::build_events::Model>, DbErr> {
        callisto::build_events::Entity::find()
            .filter(callisto::build_events::Column::TaskId.eq(task_id))
            .all(conn)
            .await
    }

    pub async fn latest_by_task_id(
        conn: &impl ConnectionTrait,
        task_id: Uuid,
    ) -> Result<Option<callisto::build_events::Model>, DbErr> {
        callisto::build_events::Entity::find()
            .filter(callisto::build_events::Column::TaskId.eq(task_id))
            .order_by_desc(callisto::build_events::Column::StartAt)
            .one(conn)
            .await
    }

    pub async fn insert_build(
        conn: &impl ConnectionTrait,
        build_id: Uuid,
        task_id: Uuid,
        repo: String,
    ) -> Result<callisto::build_events::Model, DbErr> {
        let build_model = Self::create_build_model(build_id, task_id, repo);
        build_model.insert(conn).await
    }

    pub async fn update_retry_count(
        build_id: &str,
        retry_count: i32,
        db_connection: &impl ConnectionTrait,
    ) -> Result<(), DbErr> {
        callisto::build_events::Entity::update_many()
            .filter(callisto::build_events::Column::Id.eq(build_id.parse::<Uuid>().unwrap()))
            .set(callisto::build_events::ActiveModel {
                retry_count: Set(retry_count),
                ..Default::default()
            })
            .exec(db_connection)
            .await?;
        Ok(())
    }

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

    pub async fn mark_interrupted(
        build_id: Uuid,
        end_at: sea_orm::prelude::DateTimeWithTimeZone,
        db_connection: &impl ConnectionTrait,
    ) -> Result<(), DbErr> {
        callisto::build_events::Entity::update_many()
            .filter(callisto::build_events::Column::Id.eq(build_id))
            .set(callisto::build_events::ActiveModel {
                end_at: Set(Some(end_at)),
                exit_code: Set(None),
                ..Default::default()
            })
            .exec(db_connection)
            .await?;
        Ok(())
    }
}
