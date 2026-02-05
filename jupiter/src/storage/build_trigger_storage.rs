use api_model::common::Pagination;
use callisto::build_triggers;
use common::errors::MegaError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

use crate::storage::base_storage::{BaseStorage, StorageConnector};

#[derive(Clone)]
pub struct BuildTriggerStorage {
    pub base: BaseStorage,
}

impl BuildTriggerStorage {
    /// Insert a new build trigger with optional task_id.
    /// When task_id is provided, inserts a complete record in one operation.
    pub async fn insert(
        &self,
        trigger_type: String,
        trigger_source: String,
        trigger_payload: serde_json::Value,
        task_id: Option<uuid::Uuid>,
    ) -> Result<build_triggers::Model, MegaError> {
        let now = chrono::Utc::now().naive_utc();

        let trigger = build_triggers::ActiveModel {
            id: ActiveValue::Set(common::utils::generate_id()),
            trigger_type: ActiveValue::Set(trigger_type),
            trigger_source: ActiveValue::Set(trigger_source),
            trigger_payload: ActiveValue::Set(trigger_payload),
            trigger_time: ActiveValue::Set(now),
            task_id: ActiveValue::Set(task_id),
            updated_at: ActiveValue::Set(now),
        };

        trigger
            .insert(self.base.get_connection())
            .await
            .map_err(MegaError::Db)
    }

    /// Get trigger by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<build_triggers::Model>, MegaError> {
        build_triggers::Entity::find_by_id(id)
            .one(self.base.get_connection())
            .await
            .map_err(MegaError::Db)
    }

    /// Get recent triggers (for history/audit)
    pub async fn get_recent(&self, limit: u64) -> Result<Vec<build_triggers::Model>, MegaError> {
        build_triggers::Entity::find()
            .order_by_desc(build_triggers::Column::TriggerTime)
            .limit(limit)
            .all(self.base.get_connection())
            .await
            .map_err(MegaError::Db)
    }

    /// Get triggers with pagination and filters (project standard pattern)
    pub async fn get_trigger_list(
        &self,
        params: impl Into<ListTriggersFilter>,
        page: Pagination,
    ) -> Result<(Vec<build_triggers::Model>, u64), MegaError> {
        let filter: ListTriggersFilter = params.into();
        let mut condition = Condition::all();

        // Filter by trigger_type
        if let Some(trigger_type) = filter.trigger_type {
            condition = condition.add(build_triggers::Column::TriggerType.eq(trigger_type));
        }

        // Filter by trigger_source
        if let Some(trigger_source) = filter.trigger_source {
            condition = condition.add(build_triggers::Column::TriggerSource.eq(trigger_source));
        }

        // Filter by time range
        if let Some(start_time) = filter.start_time {
            condition =
                condition.add(build_triggers::Column::TriggerTime.gte(start_time.naive_utc()));
        }
        if let Some(end_time) = filter.end_time {
            condition =
                condition.add(build_triggers::Column::TriggerTime.lte(end_time.naive_utc()));
        }

        // Note: repo_path and triggered_by are stored in JSON payload
        // For now, we'll filter them in memory after fetching
        // TODO: Consider adding indexed columns for frequently queried fields

        let query = build_triggers::Entity::find()
            .filter(condition)
            .order_by_desc(build_triggers::Column::TriggerTime);

        let paginator = query.paginate(self.base.get_connection(), page.per_page);
        let total = paginator.num_items().await.map_err(MegaError::Db)?;
        let mut items = paginator
            .fetch_page(page.page - 1)
            .await
            .map_err(MegaError::Db)?;

        // Apply JSON-based filters (repo_path, triggered_by)
        if filter.repo_path.is_some() || filter.triggered_by.is_some() {
            items.retain(|item| {
                let payload = &item.trigger_payload;

                // Filter by repo_path
                if let Some(ref repo_filter) = filter.repo_path {
                    if let Some(repo) = payload.get("repo").and_then(|v| v.as_str()) {
                        if repo != repo_filter {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Filter by triggered_by
                if let Some(ref user_filter) = filter.triggered_by {
                    if let Some(user) = payload.get("triggered_by").and_then(|v| v.as_str()) {
                        if user != user_filter {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            });
        }

        Ok((items, total))
    }
}

/// Filter parameters for listing triggers
#[derive(Debug, Default)]
pub struct ListTriggersFilter {
    pub repo_path: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_source: Option<String>,
    pub triggered_by: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::tests::test_storage;

    #[tokio::test]
    async fn test_insert_and_get() {
        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;

        let payload = serde_json::json!({
            "type": "git_push",
            "repo": "/test",
            "commit_hash": "abc123",
            "cl_link": "test_link",
            "builds": []
        });

        let inserted = storage
            .build_trigger_storage()
            .insert("git_push".to_string(), "user".to_string(), payload, None)
            .await
            .unwrap();

        let retrieved = storage
            .build_trigger_storage()
            .get_by_id(inserted.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, inserted.id);
        assert_eq!(retrieved.trigger_type, "git_push");
        assert_eq!(retrieved.trigger_source, "user");
    }

    #[tokio::test]
    async fn test_insert_with_task_id() {
        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;

        let payload = serde_json::json!({
            "type": "manual",
            "repo": "/test",
            "commit_hash": "abc123",
            "cl_link": "test_link",
            "builds": []
        });

        let task_id = uuid::Uuid::new_v4();

        let inserted = storage
            .build_trigger_storage()
            .insert(
                "manual".to_string(),
                "user".to_string(),
                payload,
                Some(task_id),
            )
            .await
            .unwrap();

        let retrieved = storage
            .build_trigger_storage()
            .get_by_id(inserted.id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.task_id, Some(task_id));
    }

    #[tokio::test]
    async fn test_get_recent() {
        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;

        let payload = serde_json::json!({
            "type": "manual",
            "repo": "/test",
            "commit_hash": "def456",
            "cl_link": "manual_link",
            "builds": []
        });

        for _ in 0..3 {
            storage
                .build_trigger_storage()
                .insert(
                    "manual".to_string(),
                    "user".to_string(),
                    payload.clone(),
                    None,
                )
                .await
                .unwrap();
        }

        let recent = storage.build_trigger_storage().get_recent(2).await.unwrap();

        assert_eq!(recent.len(), 2);
    }

    #[tokio::test]
    async fn test_get_trigger_list_with_pagination() {
        let temp_dir = tempdir().unwrap();
        let storage = test_storage(temp_dir.path()).await;

        // Insert 5 triggers
        for i in 0..5 {
            let payload = serde_json::json!({
                "type": "manual",
                "repo": "/project",
                "commit_hash": format!("hash{}", i),
                "triggered_by": "user",
                "cl_link": format!("link{}", i),
                "builds": []
            });

            storage
                .build_trigger_storage()
                .insert("manual".to_string(), "user".to_string(), payload, None)
                .await
                .unwrap();
        }

        // Get first page
        let page = Pagination {
            page: 1,
            per_page: 2,
        };
        let filter = ListTriggersFilter::default();
        let (items, total) = storage
            .build_trigger_storage()
            .get_trigger_list(filter, page)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(total, 5);

        // Get second page
        let page = Pagination {
            page: 2,
            per_page: 2,
        };
        let filter = ListTriggersFilter::default();
        let (items, total) = storage
            .build_trigger_storage()
            .get_trigger_list(filter, page)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(total, 5);

        // Get third page (only 1 item left)
        let page = Pagination {
            page: 3,
            per_page: 2,
        };
        let filter = ListTriggersFilter::default();
        let (items, total) = storage
            .build_trigger_storage()
            .get_trigger_list(filter, page)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(total, 5);
    }
}
