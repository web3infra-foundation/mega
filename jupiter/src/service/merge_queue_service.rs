use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use callisto::sea_orm_active_enums::{MergeStatusEnum, QueueFailureTypeEnum, QueueStatusEnum};
use common::errors::MegaError;

use crate::{
    model::merge_queue_dto::QueueStats,
    storage::{
        base_storage::{BaseStorage, StorageConnector},
        cl_storage::ClStorage,
        merge_queue_storage::MergeQueueStorage,
    },
};

/// Merge queue service for CL processing
#[derive(Clone)]
pub struct MergeQueueService {
    merge_queue_storage: MergeQueueStorage,
    cl_storage: ClStorage,
    processor_running: Arc<AtomicBool>,
}

impl MergeQueueService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
            merge_queue_storage: MergeQueueStorage::new(base_storage.clone()),
            cl_storage: ClStorage {
                base: base_storage.clone(),
            },
            processor_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Adds a CL to the merge queue.
    ///
    /// Note: This method only adds to queue. The background processor
    /// should be started by the caller (MonoApiService) after this call.
    pub async fn add_to_queue(&self, cl_link: String) -> Result<i64, MegaError> {
        self.validate_cl_for_queue(&cl_link).await?;

        let position = self
            .merge_queue_storage
            .add_to_queue(cl_link)
            .await
            .map_err(MegaError::Other)?;

        Ok(position)
    }

    pub async fn remove_from_queue(&self, cl_link: &str) -> Result<bool, MegaError> {
        self.merge_queue_storage
            .remove_from_queue(cl_link)
            .await
            .map_err(MegaError::Other)
    }

    pub async fn get_queue_list(&self) -> Result<Vec<callisto::merge_queue::Model>, MegaError> {
        self.merge_queue_storage
            .get_queue_list()
            .await
            .map_err(MegaError::Other)
    }

    pub async fn get_cl_queue_status(
        &self,
        cl_link: &str,
    ) -> Result<Option<callisto::merge_queue::Model>, MegaError> {
        self.merge_queue_storage
            .get_cl_queue_status(cl_link)
            .await
            .map_err(MegaError::Other)
    }

    pub async fn get_display_position(&self, cl_link: &str) -> Result<Option<usize>, MegaError> {
        self.merge_queue_storage
            .get_display_position(cl_link)
            .await
            .map_err(MegaError::Other)
    }

    pub async fn get_display_position_by_position(
        &self,
        position: i64,
    ) -> Result<usize, MegaError> {
        self.merge_queue_storage
            .get_display_position_by_position(position)
            .await
            .map_err(MegaError::Other)
    }

    pub async fn get_queue_stats(&self) -> Result<QueueStats, MegaError> {
        self.merge_queue_storage
            .get_queue_stats()
            .await
            .map_err(MegaError::Other)
    }

    // ========== Methods for MonoApiService to use ==========

    /// Gets the next waiting item from the queue.
    ///
    /// Called by MonoApiService's background processor.
    pub async fn get_next_waiting_item(
        &self,
    ) -> Result<Option<callisto::merge_queue::Model>, MegaError> {
        self.merge_queue_storage
            .get_next_waiting_item()
            .await
            .map_err(MegaError::Other)
    }

    /// Updates the status of a queue item.
    ///
    /// Returns true if update was successful, false if item was cancelled/not found.
    pub async fn update_item_status(
        &self,
        cl_link: &str,
        status: QueueStatusEnum,
    ) -> Result<bool, MegaError> {
        self.merge_queue_storage
            .update_item_status(cl_link, status)
            .await
            .map_err(MegaError::Other)
    }

    /// Updates item status to Failed with error details.
    pub async fn update_item_status_with_error(
        &self,
        cl_link: &str,
        failure_type: QueueFailureTypeEnum,
        message: String,
    ) -> Result<bool, MegaError> {
        self.merge_queue_storage
            .update_item_status_with_error(cl_link, failure_type, message)
            .await
            .map_err(MegaError::Other)
    }

    /// Moves a conflicting item to the tail of the queue for retry.
    pub async fn move_item_to_tail(&self, cl_link: &str) -> Result<bool, MegaError> {
        self.merge_queue_storage
            .move_item_to_tail(cl_link)
            .await
            .map_err(MegaError::Other)
    }

    // ========== Processor control methods ==========

    /// Tries to start the processor. Returns true if this call started it,
    /// false if it was already running.
    ///
    /// The actual processor loop should be implemented in MonoApiService (ceres layer).
    pub fn try_start_processor(&self) -> bool {
        self.processor_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Stops the processor by setting the running flag to false.
    pub fn stop_processor(&self) {
        self.processor_running.store(false, Ordering::SeqCst);
    }

    /// Checks if the processor is currently running.
    pub fn is_processor_running(&self) -> bool {
        self.processor_running.load(Ordering::SeqCst)
    }

    // ========== Validation and helper methods ==========

    /// Validates CL exists and is not closed before adding to queue
    async fn validate_cl_for_queue(&self, cl_link: &str) -> Result<(), MegaError> {
        let cl = self.cl_storage.get_cl(cl_link).await?;

        match cl {
            Some(cl_model) => match cl_model.status {
                MergeStatusEnum::Open => Ok(()),
                MergeStatusEnum::Closed => {
                    Err(MegaError::Other("Cannot queue a closed CL".to_string()))
                }
                MergeStatusEnum::Merged => {
                    Err(MegaError::Other("Cannot queue a merged CL".to_string()))
                }
                MergeStatusEnum::Draft => {
                    Err(MegaError::Other("Cannot queue a draft CL".to_string()))
                }
            },
            None => Err(MegaError::Other("CL not found".to_string())),
        }
    }

    pub async fn cancel_all_pending(&self) -> Result<u64, MegaError> {
        let count = self
            .merge_queue_storage
            .cancel_all_pending()
            .await
            .map_err(MegaError::Other)?;
        Ok(count)
    }

    /// Retries a failed queue item by resetting its status to Waiting.
    ///
    /// Note: The caller (MonoApiService) should start the processor after this.
    pub async fn retry_queue_item(&self, cl_link: &str) -> Result<bool, MegaError> {
        self.merge_queue_storage
            .retry_failed_item(cl_link)
            .await
            .map_err(MegaError::Other)
    }

    pub fn mock() -> Self {
        let base_storage = BaseStorage::mock();
        Self::new(base_storage)
    }
}
