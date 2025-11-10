use crate::model::merge_queue_dto::QueueStats;
use crate::storage::{
    base_storage::{BaseStorage, StorageConnector},
    cl_storage::ClStorage,
    merge_queue_storage::MergeQueueStorage,
};
use callisto::sea_orm_active_enums::MergeStatusEnum;
use callisto::sea_orm_active_enums::{QueueFailureTypeEnum, QueueStatusEnum};
use common::errors::MegaError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Queue polling interval in seconds when no items are processed
const QUEUE_POLL_INTERVAL_SECS: u64 = 5;

/// Error backoff interval in seconds after processing failure
const ERROR_BACKOFF_SECS: u64 = 30;

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

    pub async fn add_to_queue(&self, cl_link: String) -> Result<i64, MegaError> {
        self.validate_cl_for_queue(&cl_link).await?;

        let position = self
            .merge_queue_storage
            .add_to_queue(cl_link)
            .await
            .map_err(|e| MegaError::with_message(&e))?;

        // Start processor if not already running
        self.ensure_processor_running();

        Ok(position)
    }

    pub async fn remove_from_queue(&self, cl_link: &str) -> Result<bool, MegaError> {
        self.merge_queue_storage
            .remove_from_queue(cl_link)
            .await
            .map_err(|e| MegaError::with_message(&e))
    }

    pub async fn get_queue_list(&self) -> Result<Vec<callisto::merge_queue::Model>, MegaError> {
        self.merge_queue_storage
            .get_queue_list()
            .await
            .map_err(|e| MegaError::with_message(&e))
    }

    pub async fn get_cl_queue_status(
        &self,
        cl_link: &str,
    ) -> Result<Option<callisto::merge_queue::Model>, MegaError> {
        self.merge_queue_storage
            .get_cl_queue_status(cl_link)
            .await
            .map_err(|e| MegaError::with_message(&e))
    }

    pub async fn get_queue_stats(&self) -> Result<QueueStats, MegaError> {
        self.merge_queue_storage
            .get_queue_stats()
            .await
            .map_err(|e| MegaError::with_message(&e))
    }

    pub async fn process_next_item(&self) -> Result<bool, MegaError> {
        let next_item = self
            .merge_queue_storage
            .get_next_waiting_item()
            .await
            .map_err(|e| MegaError::with_message(&e))?;

        if let Some(item) = next_item {
            self.merge_queue_storage
                .update_item_status(&item.cl_link, QueueStatusEnum::Testing)
                .await
                .map_err(|e| MegaError::with_message(&e))?;

            let cl_link = item.cl_link.clone();

            let processed = match self.process_merge_workflow(&cl_link).await {
                Ok(()) => true,
                Err((failure_type, message)) => {
                    if matches!(failure_type, QueueFailureTypeEnum::Conflict) {
                        if let Err(e) = self.merge_queue_storage.move_item_to_tail(&cl_link).await {
                            tracing::warn!(
                                "Failed to move conflicting item {} to tail: {}",
                                cl_link,
                                e
                            );
                        }
                        false
                    } else {
                        if let Err(e) = self
                            .merge_queue_storage
                            .update_item_status_with_error(&cl_link, failure_type, message)
                            .await
                        {
                            tracing::error!(
                                "Failed to update item {} status to failed: {}",
                                cl_link,
                                e
                            );
                        }
                        true
                    }
                }
            };
            Ok(processed)
        } else {
            Ok(false)
        }
    }

    /// Orchestrates the merge workflow: validation → testing → conflict check → merge
    ///
    /// Updates queue and CL status on success or returns QueueError on failure
    async fn process_merge_workflow(
        &self,
        cl_link: &str,
    ) -> Result<(), (QueueFailureTypeEnum, String)> {
        // Validate CL still exists and is not closed before processing
        let cl = self.cl_storage.get_cl(cl_link).await.map_err(|e| {
            (
                QueueFailureTypeEnum::SystemError,
                format!("Failed to fetch CL: {}", e),
            )
        })?;

        let cl_model = match cl {
            Some(cl_model) => {
                if cl_model.status == MergeStatusEnum::Closed {
                    return Err((
                        QueueFailureTypeEnum::SystemError,
                        "CL has been closed, cannot merge".to_string(),
                    ));
                }
                cl_model
            }
            None => {
                return Err((
                    QueueFailureTypeEnum::SystemError,
                    "CL no longer exists, cannot merge".to_string(),
                ));
            }
        };

        self.execute_testing(cl_link).await?;
        self.check_conflicts(cl_link).await?;

        self.merge_queue_storage
            .update_item_status(cl_link, QueueStatusEnum::Merging)
            .await
            .map_err(|e| {
                (
                    QueueFailureTypeEnum::SystemError,
                    format!("Failed to update status to merging: {}", e),
                )
            })?;

        self.execute_merge(cl_link).await?;

        // Update merge queue status
        self.merge_queue_storage
            .update_item_status(cl_link, QueueStatusEnum::Merged)
            .await
            .map_err(|e| {
                (
                    QueueFailureTypeEnum::SystemError,
                    format!("Failed to update status to merged: {}", e),
                )
            })?;

        // Update CL status in mega_cl table
        self.cl_storage.merge_cl(cl_model).await.map_err(|e| {
            (
                QueueFailureTypeEnum::SystemError,
                format!("Failed to update CL status: {}", e),
            )
        })?;

        Ok(())
    }

    /// Validates CL exists and is not closed before adding to queue
    async fn validate_cl_for_queue(&self, cl_link: &str) -> Result<(), MegaError> {
        let cl = self.cl_storage.get_cl(cl_link).await?;

        match cl {
            Some(cl_model) => match cl_model.status {
                MergeStatusEnum::Open => Ok(()),
                MergeStatusEnum::Closed => Err(MegaError::with_message("Cannot queue a closed CL")),
                MergeStatusEnum::Merged => Err(MegaError::with_message("Cannot queue a merged CL")),
            },
            None => Err(MegaError::with_message("CL not found")),
        }
    }

    /// Checks for merge conflicts using Git internals
    ///
    /// TODO: Implement actual Git conflict detection
    async fn check_conflicts(&self, _cl_link: &str) -> Result<(), (QueueFailureTypeEnum, String)> {
        Ok(())
    }

    /// Executes Git merge operation for the CL
    ///
    /// TODO: Implement actual Git merge operation
    async fn execute_merge(&self, _cl_link: &str) -> Result<(), (QueueFailureTypeEnum, String)> {
        Ok(())
    }

    /// Executes Buck2 tests for the CL and returns error if tests fail
    async fn execute_testing(&self, cl_link: &str) -> Result<(), (QueueFailureTypeEnum, String)> {
        let cl = self
            .cl_storage
            .get_cl(cl_link)
            .await
            .map_err(|e| (QueueFailureTypeEnum::SystemError, e.to_string()))?
            .ok_or_else(|| {
                (
                    QueueFailureTypeEnum::SystemError,
                    "CL not found".to_string(),
                )
            })?;

        match self.run_buck2_tests(&cl).await {
            Ok(success) => {
                if success {
                    Ok(())
                } else {
                    Err((
                        QueueFailureTypeEnum::TestFailure,
                        "Buck2 tests failed".to_string(),
                    ))
                }
            }
            Err((failure_type, message)) => Err((
                failure_type,
                format!("Buck2 test execution error: {}", message),
            )),
        }
    }

    /// Runs Buck2 tests for the CL
    ///
    /// Returns Ok(true) if tests pass, Ok(false) if tests fail
    ///
    /// TODO: Implement actual Buck2 test execution
    async fn run_buck2_tests(
        &self,
        _cl: &callisto::mega_cl::Model,
    ) -> Result<bool, (QueueFailureTypeEnum, String)> {
        Ok(true)
    }

    pub async fn cancel_all_pending(&self) -> Result<u64, MegaError> {
        let count = self
            .merge_queue_storage
            .cancel_all_pending()
            .await
            .map_err(|e| MegaError::with_message(&e))?;
        Ok(count)
    }

    /// Ensures the background merge processor is running
    ///
    /// Uses atomic flag to guarantee only one processor task runs at a time.
    /// Processor automatically stops when no active items remain in queue.
    fn ensure_processor_running(&self) {
        if self
            .processor_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let service = self.clone();
            tokio::spawn(async move {
                tracing::info!("Merge queue processor started");

                loop {
                    match service.process_next_item().await {
                        Ok(processed) => {
                            if !processed {
                                // Check if there are active items
                                if let Ok(stats) =
                                    service.merge_queue_storage.get_queue_stats().await
                                {
                                    let has_active = stats.waiting_count > 0
                                        || stats.testing_count > 0
                                        || stats.merging_count > 0;

                                    if !has_active {
                                        // No active items, stop processor
                                        service.processor_running.store(false, Ordering::SeqCst);
                                        tracing::info!(
                                            "Merge queue processor stopped (no active items)"
                                        );
                                        break;
                                    }
                                }
                                tokio::time::sleep(Duration::from_secs(QUEUE_POLL_INTERVAL_SECS))
                                    .await;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Queue processor error: {}", e);
                            tokio::time::sleep(Duration::from_secs(ERROR_BACKOFF_SECS)).await;
                        }
                    }
                }
            });
        }
    }

    pub async fn retry_queue_item(&self, cl_link: &str) -> Result<bool, MegaError> {
        let result = self
            .merge_queue_storage
            .retry_failed_item(cl_link)
            .await
            .map_err(|e| MegaError::with_message(&e))?;

        if result {
            self.ensure_processor_running();
        }

        Ok(result)
    }

    pub fn mock() -> Self {
        let base_storage = BaseStorage::mock();
        Self::new(base_storage)
    }
}
