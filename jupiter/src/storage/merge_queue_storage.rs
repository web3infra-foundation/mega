use crate::model::merge_queue::{QueueError, QueueItem, QueueStatus};
use crate::storage::base_storage::{BaseStorage, StorageConnector};
use callisto::mega_conversation::{Column, Entity};
use callisto::sea_orm_active_enums::ConvTypeEnum;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::ops::Deref;

/// Username for merge queue system operations
const MERGE_QUEUE_USERNAME: &str = "system";

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

    /// Constructs base query with common filters for merge queue items
    ///
    /// Filters by ConvType=MergeQueue and Username=system
    fn base_queue_query(&self) -> sea_orm::Select<callisto::mega_conversation::Entity> {
        Entity::find()
            .filter(Column::ConvType.eq(ConvTypeEnum::MergeQueue))
            .filter(Column::Username.eq(MERGE_QUEUE_USERNAME))
    }

    /// Adds CL to queue with timestamp position
    pub async fn add_to_queue(&self, cl_link: String) -> Result<i32, String> {
        let db = self.get_connection();

        // Check if CL is already in queue (any status)
        let existing = self
            .base_queue_query()
            .filter(Column::Link.eq(&cl_link))
            .one(db)
            .await
            .map_err(|e| format!("Failed to check existing CL: {}", e))?;

        if let Some(conv) = existing {
            // Deserialize to check status
            if let Some(comment) = &conv.comment
                && let Ok(item) = serde_json::from_str::<QueueItem>(comment)
            {
                return match item.status {
                    QueueStatus::Waiting | QueueStatus::Testing | QueueStatus::Merging => Err(
                        format!("CL is already in the queue with status {:?}", item.status),
                    ),
                    QueueStatus::Merged => {
                        Err("CL has already been merged, cannot add to queue again".to_string())
                    }
                    QueueStatus::Failed => Err(
                        "CL previously failed, please use retry endpoint instead of adding again"
                            .to_string(),
                    ),
                };
            }
            // Fallback: if cannot deserialize, still block
            return Err("CL already exists in queue records".to_string());
        }

        // Use timestamp as position for consistent ordering
        let position = chrono::Utc::now().timestamp() as i32;
        let now = chrono::Utc::now();

        // Create new queue item
        let new_item = QueueItem::new(cl_link.clone(), position);
        let serialized = serde_json::to_string(&new_item)
            .map_err(|e| format!("Failed to serialize queue item: {}", e))?;

        // Insert operation
        let conversation = callisto::mega_conversation::ActiveModel {
            id: Set(common::utils::generate_id()),
            link: Set(cl_link),
            conv_type: Set(ConvTypeEnum::MergeQueue),
            comment: Set(Some(serialized)),
            username: Set(MERGE_QUEUE_USERNAME.to_string()),
            resolved: Set(None),
            created_at: Set(now.naive_utc()),
            updated_at: Set(now.naive_utc()),
        };

        conversation
            .insert(db)
            .await
            .map_err(|e| format!("Failed to insert queue item: {}", e))?;

        Ok(position)
    }

    pub async fn remove_from_queue(&self, cl_link: &str) -> Result<bool, String> {
        let db = self.get_connection();

        // Check status before removing
        let existing = self
            .base_queue_query()
            .filter(Column::Link.eq(cl_link))
            .one(db)
            .await
            .map_err(|e| format!("Failed to check existing CL: {}", e))?;

        if let Some(conv) = existing {
            if let Some(comment) = &conv.comment
                && let Ok(item) = serde_json::from_str::<QueueItem>(comment)
            {
                // Prevent removing items that are currently being processed
                if matches!(item.status, QueueStatus::Testing | QueueStatus::Merging) {
                    return Err(format!(
                        "Cannot remove CL with status {:?}, it is currently being processed",
                        item.status
                    ));
                }
            }
        } else {
            return Ok(false); // Item not found
        }

        let delete_result = Entity::delete_many()
            .filter(Column::ConvType.eq(ConvTypeEnum::MergeQueue))
            .filter(Column::Username.eq(MERGE_QUEUE_USERNAME))
            .filter(Column::Link.eq(cl_link))
            .exec(db)
            .await
            .map_err(|e| format!("Failed to remove queue item: {}", e))?;

        Ok(delete_result.rows_affected > 0)
    }

    pub async fn get_queue_list(&self) -> Result<Vec<QueueItem>, String> {
        let db = self.get_connection();

        let conversations = self
            .base_queue_query()
            .filter(
                Column::Comment
                    .like("%\"status\":\"waiting\"%")
                    .or(Column::Comment.like("%\"status\":\"testing\"%"))
                    .or(Column::Comment.like("%\"status\":\"merging\"%")),
            )
            .order_by_asc(Column::CreatedAt)
            .all(db)
            .await
            .map_err(|e| format!("Failed to fetch queue items: {}", e))?;

        let mut items = Vec::new();
        for conv in conversations {
            if let Some(comment) = &conv.comment {
                match serde_json::from_str::<QueueItem>(comment) {
                    Ok(item) => {
                        // Double-check status after deserialization
                        if matches!(
                            item.status,
                            QueueStatus::Waiting | QueueStatus::Testing | QueueStatus::Merging
                        ) {
                            items.push(item);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize queue item {}: {}", conv.id, e);
                        continue;
                    }
                }
            }
        }

        // Sort by position
        items.sort_by_key(|item| item.position);

        Ok(items)
    }

    pub async fn get_cl_queue_status(&self, cl_link: &str) -> Result<Option<QueueItem>, String> {
        let db = self.get_connection();

        let conversation = self
            .base_queue_query()
            .filter(Column::Link.eq(cl_link))
            .one(db)
            .await
            .map_err(|e| format!("Failed to find item by link: {}", e))?;

        if let Some(conv) = conversation {
            if let Some(comment) = &conv.comment {
                match serde_json::from_str::<QueueItem>(comment) {
                    Ok(item) => Ok(Some(item)),
                    Err(e) => Err(format!("Failed to deserialize queue item: {}", e)),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_next_waiting_item(&self) -> Result<Option<QueueItem>, String> {
        let db = self.get_connection();

        let conversations = self
            .base_queue_query()
            .filter(Column::Comment.like("%\"status\":\"waiting\"%"))
            .order_by_asc(Column::CreatedAt)
            .all(db)
            .await
            .map_err(|e| format!("Failed to find waiting items: {}", e))?;

        let mut next_item: Option<(QueueItem, chrono::NaiveDateTime)> = None;

        for conv in conversations {
            if let Some(ref comment) = conv.comment {
                match serde_json::from_str::<QueueItem>(comment) {
                    Ok(item) => {
                        if item.status != QueueStatus::Waiting {
                            continue;
                        }

                        let created_at = conv.created_at;
                        match &next_item {
                            Some((current_item, current_created_at)) => {
                                if item.position < current_item.position
                                    || (item.position == current_item.position
                                        && created_at < *current_created_at)
                                {
                                    next_item = Some((item, created_at));
                                }
                            }
                            None => {
                                next_item = Some((item, created_at));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize queue item: {}", e);
                        continue;
                    }
                }
            }
        }

        Ok(next_item.map(|(item, _)| item))
    }

    /// Atomically updates a queue item using a transaction
    ///
    /// The modifier function receives a mutable reference to the item.
    /// Returns Ok(true) if updated, Ok(false) if not found, Err on failure.
    /// Transaction is rolled back on any error to prevent partial updates.
    async fn update_queue_item<F>(&self, cl_link: &str, modifier: F) -> Result<bool, String>
    where
        F: FnOnce(&mut QueueItem) -> Result<(), String>,
    {
        let db = self.get_connection();

        let txn = db
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        let existing = Entity::find()
            .filter(Column::ConvType.eq(ConvTypeEnum::MergeQueue))
            .filter(Column::Username.eq(MERGE_QUEUE_USERNAME))
            .filter(Column::Link.eq(cl_link))
            .one(&txn)
            .await
            .map_err(|e| format!("Failed to find item: {}", e))?;

        if let Some(model) = existing {
            if let Some(comment) = &model.comment {
                match serde_json::from_str::<QueueItem>(comment) {
                    Ok(mut item) => {
                        modifier(&mut item)?;

                        let updated_serialized = serde_json::to_string(&item)
                            .map_err(|e| format!("Failed to serialize updated item: {}", e))?;

                        let mut active_model: callisto::mega_conversation::ActiveModel =
                            model.into();
                        active_model.comment = Set(Some(updated_serialized));
                        active_model.updated_at = Set(chrono::Utc::now().naive_utc());

                        active_model
                            .update(&txn)
                            .await
                            .map_err(|e| format!("Failed to update item: {}", e))?;

                        txn.commit()
                            .await
                            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

                        Ok(true)
                    }
                    Err(e) => {
                        let _ = txn.rollback().await;
                        Err(format!("Failed to deserialize queue item: {}", e))
                    }
                }
            } else {
                let _ = txn.rollback().await;
                Err("Queue item has no comment data".to_string())
            }
        } else {
            let _ = txn.rollback().await;
            Ok(false)
        }
    }

    pub async fn update_item_status(
        &self,
        cl_link: &str,
        new_status: QueueStatus,
    ) -> Result<bool, String> {
        self.update_queue_item(cl_link, |item| {
            item.update_status(new_status)
                .map_err(|e| format!("Failed to update status: {}", e))
        })
        .await
    }

    pub async fn update_item_status_with_error(
        &self,
        cl_link: &str,
        new_status: QueueStatus,
        error: QueueError,
    ) -> Result<bool, String> {
        self.update_queue_item(cl_link, |item| {
            item.update_status_with_error(new_status, error)
                .map_err(|e| format!("Failed to update status with error: {}", e))
        })
        .await
    }

    /// Gets queue statistics (optimized with single query)
    pub async fn get_queue_stats(&self) -> Result<crate::model::merge_queue::QueueStats, String> {
        let db = self.get_connection();

        // Single query to get all items
        let conversations = self
            .base_queue_query()
            .all(db)
            .await
            .map_err(|e| format!("Failed to fetch queue items: {}", e))?;

        let mut stats = crate::model::merge_queue::QueueStats {
            total_items: conversations.len(),
            waiting_count: 0,
            testing_count: 0,
            merging_count: 0,
            failed_count: 0,
            merged_count: 0,
        };

        // Count in memory
        for conv in conversations {
            if let Some(comment) = &conv.comment
                && let Ok(item) = serde_json::from_str::<QueueItem>(comment)
            {
                match item.status {
                    QueueStatus::Waiting => stats.waiting_count += 1,
                    QueueStatus::Testing => stats.testing_count += 1,
                    QueueStatus::Merging => stats.merging_count += 1,
                    QueueStatus::Failed => stats.failed_count += 1,
                    QueueStatus::Merged => stats.merged_count += 1,
                }
            }
        }

        Ok(stats)
    }

    pub async fn cancel_all_pending(&self) -> Result<u64, String> {
        let db = self.get_connection();

        let items_to_cancel = self
            .base_queue_query()
            .filter(
                Column::Comment
                    .like("%\"status\":\"waiting\"%")
                    .or(Column::Comment.like("%\"status\":\"testing\"%")),
            )
            .all(db)
            .await
            .map_err(|e| format!("Failed to find items to cancel: {}", e))?;

        let error = QueueError::new(
            crate::model::merge_queue::FailureType::SystemError,
            "Operation cancelled by user".to_string(),
        );

        // Process all items concurrently
        let update_futures: Vec<_> = items_to_cancel
            .into_iter()
            .filter_map(|model| {
                let comment = model.comment.as_ref()?;
                let item: QueueItem = serde_json::from_str(comment).ok()?;

                // Only process waiting or testing items
                if !matches!(item.status, QueueStatus::Waiting | QueueStatus::Testing) {
                    return None;
                }

                Some((model, item))
            })
            .map(|(model, mut item)| {
                let db = db.clone();
                let error = error.clone();
                async move {
                    item.update_status_with_error(QueueStatus::Failed, error)
                        .map_err(|e| format!("Failed to update cancelled item: {}", e))?;

                    let updated_serialized = serde_json::to_string(&item)
                        .map_err(|e| format!("Failed to serialize cancelled item: {}", e))?;

                    let mut active_model: callisto::mega_conversation::ActiveModel = model.into();
                    active_model.comment = Set(Some(updated_serialized));
                    active_model.updated_at = Set(chrono::Utc::now().naive_utc());

                    active_model
                        .update(&db)
                        .await
                        .map_err(|e| format!("Failed to update cancelled item: {}", e))?;

                    Ok::<_, String>(())
                }
            })
            .collect();

        // Execute all updates concurrently
        let results = futures::future::join_all(update_futures).await;

        // Count successful updates
        let cancelled_count = results.iter().filter(|r| r.is_ok()).count() as u64;

        // Log any failures
        for (idx, result) in results.iter().enumerate() {
            if let Err(e) = result {
                tracing::warn!("Failed to cancel item {}: {}", idx, e);
            }
        }

        Ok(cancelled_count)
    }

    pub fn mock() -> Self {
        let base_storage = BaseStorage::mock();
        Self::new(base_storage)
    }

    pub async fn retry_failed_item(&self, cl_link: &str) -> Result<bool, String> {
        self.update_queue_item(cl_link, |item| {
            if !matches!(item.status, QueueStatus::Failed) {
                return Err("Item is not in failed state".to_string());
            }

            if !item.can_retry(MAX_RETRY_ATTEMPTS) {
                return Err("Item has exceeded maximum retry attempts".to_string());
            }

            item.increment_retry();
            item.update_status(QueueStatus::Waiting)
                .map_err(|e| format!("Failed to update status: {}", e))?;

            item.position = chrono::Utc::now().timestamp() as i32;

            Ok(())
        })
        .await
    }

    pub async fn move_item_to_tail(&self, cl_link: &str) -> Result<bool, String> {
        let db = self.get_connection();

        // Use transaction to prevent race condition
        let txn = db
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        // Query all items within transaction
        let conversations = Entity::find()
            .filter(Column::ConvType.eq(ConvTypeEnum::MergeQueue))
            .filter(Column::Username.eq(MERGE_QUEUE_USERNAME))
            .all(&txn)
            .await
            .map_err(|e| format!("Failed to query queue items: {}", e))?;

        let mut max_position = chrono::Utc::now().timestamp() as i32;

        for conv in &conversations {
            if let Some(comment) = &conv.comment {
                match serde_json::from_str::<QueueItem>(comment) {
                    Ok(item) => {
                        if item.cl_link != cl_link && item.position >= max_position {
                            max_position = item.position + 1;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize queue item: {}", e);
                        continue;
                    }
                }
            }
        }

        // Find and update the target item within the same transaction
        let target = conversations.into_iter().find(|conv| conv.link == cl_link);

        if let Some(model) = target {
            if let Some(comment) = &model.comment {
                match serde_json::from_str::<QueueItem>(comment) {
                    Ok(mut item) => {
                        item.update_status(QueueStatus::Waiting)
                            .map_err(|e| format!("Failed to update status: {}", e))?;
                        item.position = max_position;

                        let updated_serialized = serde_json::to_string(&item)
                            .map_err(|e| format!("Failed to serialize updated item: {}", e))?;

                        let mut active_model: callisto::mega_conversation::ActiveModel =
                            model.into();
                        active_model.comment = Set(Some(updated_serialized));
                        active_model.updated_at = Set(chrono::Utc::now().naive_utc());

                        active_model
                            .update(&txn)
                            .await
                            .map_err(|e| format!("Failed to update item: {}", e))?;

                        txn.commit()
                            .await
                            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

                        Ok(true)
                    }
                    Err(e) => {
                        let _ = txn.rollback().await;
                        Err(format!("Failed to deserialize queue item: {}", e))
                    }
                }
            } else {
                let _ = txn.rollback().await;
                Err("Queue item has no comment data".to_string())
            }
        } else {
            let _ = txn.rollback().await;
            Ok(false)
        }
    }
}
