use crate::storage::base_storage::{BaseStorage, StorageConnector};
use callisto::merge_queue::{ActiveModel, Column, Entity, Model};
use callisto::sea_orm_active_enums::{QueueFailureTypeEnum, QueueStatusEnum};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::ops::Deref;

/// Maximum number of retry attempts for failed items
const MAX_RETRY_ATTEMPTS: i32 = 3;

/// Merge queue storage layer
#[derive(Clone)]
pub struct MergeQueueStorage {
    base: BaseStorage,
}

impl Deref for MergeQueueStorage {
    type Target = BaseStorage;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl MergeQueueStorage {
    pub fn new(base: BaseStorage) -> Self {
        Self { base }
    }

    /// Adds CL to queue with timestamp position
    pub async fn add_to_queue(&self, cl_link: String) -> Result<i64, String> {
        let db = self.get_connection();

        // Check if CL is already in queue (any status)
        let existing = Entity::find()
            .filter(Column::ClLink.eq(&cl_link))
            .one(db)
            .await
            .map_err(|e| format!("Failed to check existing CL: {}", e))?;

        if let Some(item) = existing {
            return match item.status {
                QueueStatusEnum::Waiting | QueueStatusEnum::Testing | QueueStatusEnum::Merging => {
                    Err(format!(
                        "CL is already in the queue with status {:?}",
                        item.status
                    ))
                }
                QueueStatusEnum::Merged => {
                    Err("CL has already been merged, cannot add to queue again".to_string())
                }
                QueueStatusEnum::Failed => Err(
                    "CL previously failed, please use retry endpoint instead of adding again"
                        .to_string(),
                ),
            };
        }

        // Use timestamp as position value
        let now = chrono::Utc::now();
        let position = now.timestamp_millis();

        // Create new queue item
        let new_item = ActiveModel {
            id: Set(common::utils::generate_id()),
            cl_link: Set(cl_link),
            status: Set(QueueStatusEnum::Waiting),
            position: Set(position),
            retry_count: Set(0),
            last_retry_at: Set(None),
            failure_type: Set(None),
            error_message: Set(None),
            created_at: Set(now.naive_utc()),
            updated_at: Set(now.naive_utc()),
        };

        new_item
            .insert(db)
            .await
            .map_err(|e| format!("Failed to insert queue item: {}", e))?;

        Ok(position)
    }

    pub async fn remove_from_queue(&self, cl_link: &str) -> Result<bool, String> {
        let db = self.get_connection();

        // Check status before removing
        let existing = Entity::find()
            .filter(Column::ClLink.eq(cl_link))
            .one(db)
            .await
            .map_err(|e| format!("Failed to check existing CL: {}", e))?;

        if let Some(item) = existing {
            if matches!(
                item.status,
                QueueStatusEnum::Testing | QueueStatusEnum::Merging
            ) {
                return Err(format!(
                    "Cannot remove CL with status {:?}, it is currently being processed",
                    item.status
                ));
            }

            let delete_result = Entity::delete_by_id(item.id)
                .exec(db)
                .await
                .map_err(|e| format!("Failed to remove queue item: {}", e))?;

            Ok(delete_result.rows_affected > 0)
        } else {
            Ok(false)
        }
    }

    pub async fn get_queue_list(&self) -> Result<Vec<Model>, String> {
        let db = self.get_connection();

        let items = Entity::find()
            .filter(Column::Status.is_in([
                QueueStatusEnum::Waiting,
                QueueStatusEnum::Testing,
                QueueStatusEnum::Merging,
            ]))
            .order_by_asc(Column::Position)
            .all(db)
            .await
            .map_err(|e| format!("Failed to fetch queue items: {}", e))?;

        Ok(items)
    }

    pub async fn get_cl_queue_status(&self, cl_link: &str) -> Result<Option<Model>, String> {
        Entity::find()
            .filter(Column::ClLink.eq(cl_link))
            .one(self.get_connection())
            .await
            .map_err(|e| format!("Failed to find item by link: {}", e))
    }

    pub async fn get_next_waiting_item(&self) -> Result<Option<Model>, String> {
        Entity::find()
            .filter(Column::Status.eq(QueueStatusEnum::Waiting))
            .order_by_asc(Column::Position) // 数据库排序
            .one(self.get_connection()) // 只获取第一个
            .await
            .map_err(|e| format!("Failed to find waiting items: {}", e))
    }

    pub async fn update_item_status(
        &self,
        cl_link: &str,
        new_status: QueueStatusEnum,
    ) -> Result<bool, String> {
        let db = self.get_connection();

        let item_model = self.find_item_by_cl_link(cl_link).await?;

        if let Some(item_model) = item_model {
            let mut active_model: ActiveModel = item_model.into();

            active_model.status = Set(new_status);
            active_model.updated_at = Set(chrono::Utc::now().naive_utc());
            active_model.error_message = Set(None);
            active_model.failure_type = Set(None);

            active_model
                .update(db)
                .await
                .map_err(|e| format!("Failed to update item: {}", e))?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn find_item_by_cl_link(&self, cl_link: &str) -> Result<Option<Model>, String> {
        Entity::find()
            .filter(Column::ClLink.eq(cl_link))
            .one(self.get_connection())
            .await
            .map_err(|e| format!("Failed to find item by cl link: {}", e))
    }

    pub async fn update_item_status_with_error(
        &self,
        cl_link: &str,
        failure_type: QueueFailureTypeEnum,
        error: String,
    ) -> Result<bool, String> {
        let db = self.get_connection();

        let item_model = self.find_item_by_cl_link(cl_link).await?;

        if let Some(item_model) = item_model {
            let mut active_model: ActiveModel = item_model.into();

            active_model.status = Set(QueueStatusEnum::Failed);
            active_model.failure_type = Set(Some(failure_type));
            active_model.error_message = Set(Some(error));
            active_model.updated_at = Set(chrono::Utc::now().naive_utc());

            active_model
                .update(db)
                .await
                .map_err(|e| format!("Failed to update item: {}", e))?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Gets queue statistics (optimized with single query)
    pub async fn get_queue_stats(
        &self,
    ) -> Result<crate::model::merge_queue_dto::QueueStats, String> {
        let db = self.get_connection();

        let results: Vec<(QueueStatusEnum, i64)> = Entity::find()
            .select_only()
            .column(Column::Status)
            .column_as(Column::Status.count(), "count")
            .group_by(Column::Status)
            .into_tuple()
            .all(db)
            .await
            .map_err(|e| format!("Failed to fetch stats: {}", e))?;

        let mut stats = crate::model::merge_queue_dto::QueueStats::default();
        let mut total_items = 0;

        for (status, count) in results {
            let count_usize = count as usize;
            match status {
                QueueStatusEnum::Waiting => stats.waiting_count = count_usize,
                QueueStatusEnum::Testing => stats.testing_count = count_usize,
                QueueStatusEnum::Merging => stats.merging_count = count_usize,
                QueueStatusEnum::Failed => stats.failed_count = count_usize,
                QueueStatusEnum::Merged => stats.merged_count = count_usize,
            }
            total_items += count_usize;
        }
        stats.total_items = total_items;

        Ok(stats)
    }

    pub async fn cancel_all_pending(&self) -> Result<u64, String> {
        let db = self.get_connection();

        let items = Entity::find()
            .filter(Column::Status.is_in([QueueStatusEnum::Waiting, QueueStatusEnum::Testing]))
            .all(db)
            .await
            .map_err(|e| format!("Failed to find items to cancel: {}", e))?;

        if items.is_empty() {
            tracing::info!("No pending items to cancel");
            return Ok(0);
        }

        let now = chrono::Utc::now().naive_utc();
        let mut affected: u64 = 0;

        for item in items {
            let mut active: ActiveModel = item.into();
            active.status = Set(QueueStatusEnum::Failed);
            active.failure_type = Set(Some(QueueFailureTypeEnum::SystemError));
            active.error_message = Set(Some("Operation cancelled by user".to_string()));
            active.updated_at = Set(now);

            active
                .update(db)
                .await
                .map_err(|e| format!("Failed to cancel item: {}", e))?;

            affected += 1;
        }

        tracing::info!("Successfully cancelled {} pending items", affected);
        Ok(affected)
    }

    pub fn mock() -> Self {
        let base_storage = BaseStorage::mock();
        Self::new(base_storage)
    }

    pub async fn retry_failed_item(&self, cl_link: &str) -> Result<bool, String> {
        let db = self.get_connection();

        let item = Entity::find()
            .filter(Column::ClLink.eq(cl_link))
            .one(db)
            .await
            .map_err(|e| format!("Failed to find item: {}", e))?;

        if let Some(item) = item {
            if !matches!(item.status, QueueStatusEnum::Failed) {
                return Err("Item is not in failed state".to_string());
            }
            if item.retry_count >= MAX_RETRY_ATTEMPTS {
                return Err("Item has exceeded maximum retry attempts".to_string());
            }

            let mut active_model: ActiveModel = item.into();
            active_model.status = Set(QueueStatusEnum::Waiting);
            active_model.retry_count = Set(active_model.retry_count.unwrap() + 1);
            active_model.last_retry_at = Set(Some(chrono::Utc::now().naive_utc()));
            active_model.position = Set(chrono::Utc::now().timestamp_millis());
            active_model.failure_type = Set(None);
            active_model.error_message = Set(None);

            active_model
                .update(db)
                .await
                .map_err(|e| format!("Failed to update item for retry: {}", e))?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn move_item_to_tail(&self, cl_link: &str) -> Result<bool, String> {
        let db = self.get_connection();

        let item = Entity::find()
            .filter(Column::ClLink.eq(cl_link))
            .one(db)
            .await
            .map_err(|e| format!("Failed to find item: {}", e))?;

        if let Some(item) = item {
            let mut active_model: ActiveModel = item.into();
            active_model.status = Set(QueueStatusEnum::Waiting);
            active_model.position = Set(chrono::Utc::now().timestamp_millis());
            active_model.updated_at = Set(chrono::Utc::now().naive_utc());

            active_model
                .update(db)
                .await
                .map_err(|e| format!("Failed to update item: {}", e))?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
