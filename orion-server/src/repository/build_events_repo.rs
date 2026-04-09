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
        let completion = Self::build_completion_update(exit_code, Utc::now().into());
        callisto::build_events::Entity::update_many()
            .filter(callisto::build_events::Column::Id.eq(build_id.parse::<Uuid>().unwrap()))
            .set(completion)
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

    fn build_completion_update(
        exit_code: Option<i32>,
        end_at: sea_orm::prelude::DateTimeWithTimeZone,
    ) -> callisto::build_events::ActiveModel {
        callisto::build_events::ActiveModel {
            exit_code: Set(exit_code),
            end_at: Set(Some(end_at)),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sea_orm::ActiveValue::{NotSet, Set};
    use uuid::Uuid;

    use super::BuildEventsRepo;

    #[test]
    fn test_build_completion_update_sets_exit_code_and_end_at() {
        let end_at = Utc::now().into();
        let model = BuildEventsRepo::build_completion_update(Some(0), end_at);

        assert!(matches!(&model.exit_code, Set(Some(0))));
        assert!(matches!(&model.end_at, Set(Some(value)) if *value == end_at));
        assert!(matches!(&model.retry_count, NotSet));
    }

    #[test]
    fn test_create_build_model_uses_repo_leaf_for_log_output_key() {
        let build_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let model = BuildEventsRepo::create_build_model(
            build_id,
            task_id,
            "/project/buck2_test/".to_string(),
        );

        assert!(matches!(&model.id, Set(id) if *id == build_id));
        assert!(matches!(&model.task_id, Set(id) if *id == task_id));
        assert!(matches!(&model.log_output_file, Set(path)
            if path == &format!("{}/{}/{}.log", task_id, "buck2_test", build_id)));
    }
}
