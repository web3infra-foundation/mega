//! Merge queue background processor for [`ClApplicationService`](super::service::ClApplicationService).

use std::time::Duration;

use callisto::sea_orm_active_enums::{MergeStatusEnum, QueueFailureTypeEnum, QueueStatusEnum};
use common::errors::MegaError;
use tracing;

use crate::{
    application::api_service::mono::ClApplicationService,
    model::merge_queue::{
        AddToQueueResponse, QueueItem, QueueListResponse, QueueStatsResponse, QueueStatus,
        QueueStatusResponse,
    },
};

impl ClApplicationService {
    // ========== Merge Queue Methods ==========

    /// Queue polling interval in seconds when no items are processed
    const QUEUE_POLL_INTERVAL_SECS: u64 = 5;

    /// Error backoff interval in seconds after processing failure
    const ERROR_BACKOFF_SECS: u64 = 30;

    /// Adds a CL to the merge queue and ensures the background processor is running.
    ///
    /// This method validates the CL status before adding to queue and automatically
    /// starts the background processor if not already running.
    ///
    /// # Arguments
    /// * `cl_link` - The unique identifier of the CL to add to queue
    ///
    /// # Returns
    /// * `Ok(i64)` - The position in queue on success
    /// * `Err(MegaError)` - If validation fails or database error occurs
    pub async fn add_to_merge_queue(&self, cl_link: String) -> Result<i64, MegaError> {
        // Validate CL exists and is in Open status
        let cl = self
            .storage()
            .cl_service
            .cl_store()
            .get_cl(&cl_link)
            .await?;
        let model = cl.ok_or(MegaError::Other("CL not found".to_string()))?;

        if model.status != MergeStatusEnum::Open {
            return Err(MegaError::Other(format!(
                "CL is not in Open status, current status: {:?}",
                model.status
            )));
        }

        // Add to queue via jupiter layer service
        let position = self
            .storage()
            .merge_queue_service
            .add_to_queue(cl_link)
            .await?;

        // Ensure the background processor is running
        self.ensure_merge_processor_running();

        Ok(position)
    }

    /// Retries a failed merge queue item and ensures the processor is running.
    ///
    /// # Arguments
    /// * `cl_link` - The unique identifier of the CL to retry
    ///
    /// # Returns
    /// * `Ok(true)` - If retry was successful
    /// * `Ok(false)` - If item not found or cannot be retried
    /// * `Err(MegaError)` - If database error occurs
    pub async fn retry_merge_queue_item(&self, cl_link: &str) -> Result<bool, MegaError> {
        let result = self
            .storage()
            .merge_queue_service
            .retry_queue_item(cl_link)
            .await?;

        if result {
            // Ensure the background processor is running
            self.ensure_merge_processor_running();
        }

        Ok(result)
    }

    /// Adds a CL to the merge queue and returns API response fields including display position.
    pub async fn add_to_merge_queue_response(
        &self,
        cl_link: String,
    ) -> Result<AddToQueueResponse, MegaError> {
        let position = self.add_to_merge_queue(cl_link.clone()).await?;
        let display_position = self
            .storage()
            .merge_queue_service
            .get_display_position_by_position(position)
            .await
            .map(Some)
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to get display position after add for {}: {}",
                    cl_link,
                    e
                );
                None
            });

        Ok(AddToQueueResponse {
            success: true,
            position,
            display_position,
            message: "Added to queue".to_string(),
        })
    }

    pub async fn remove_from_merge_queue(&self, cl_link: &str) -> Result<bool, MegaError> {
        self.storage()
            .merge_queue_service
            .remove_from_queue(cl_link)
            .await
    }

    pub async fn get_merge_queue_list(&self) -> Result<QueueListResponse, MegaError> {
        let items = self.storage().merge_queue_service.get_queue_list().await?;
        Ok(QueueListResponse::from(items))
    }

    pub async fn get_cl_queue_status(
        &self,
        cl_link: &str,
    ) -> Result<QueueStatusResponse, MegaError> {
        let item_model = self
            .storage()
            .merge_queue_service
            .get_cl_queue_status(cl_link)
            .await?;

        let mut item_opt: Option<QueueItem> = item_model.map(|m| m.into());

        if let Some(ref mut item) = item_opt {
            match item.status {
                QueueStatus::Waiting | QueueStatus::Testing | QueueStatus::Merging => {
                    match self
                        .storage()
                        .merge_queue_service
                        .get_display_position(&item.cl_link)
                        .await
                    {
                        Ok(index) => item.display_position = index,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to get display position for {}: {}",
                                item.cl_link,
                                e
                            );
                            item.display_position = None;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(QueueStatusResponse {
            in_queue: item_opt.is_some(),
            item: item_opt,
        })
    }

    pub async fn get_merge_queue_stats(&self) -> Result<QueueStatsResponse, MegaError> {
        let stats = self.storage().merge_queue_service.get_queue_stats().await?;
        Ok(QueueStatsResponse::from(stats))
    }

    pub async fn cancel_all_pending_merge_queue(&self) -> Result<(), MegaError> {
        self.storage()
            .merge_queue_service
            .cancel_all_pending()
            .await?;
        Ok(())
    }

    /// Ensures the background merge processor is running.
    ///
    /// Uses atomic flag to guarantee only one processor task runs at a time.
    /// The processor automatically stops when no active items remain in queue.
    fn ensure_merge_processor_running(&self) {
        // Get the processor running flag from merge queue service
        if self.storage().merge_queue_service.try_start_processor() {
            let service = self.clone();
            tokio::spawn(async move {
                tracing::info!("Merge queue processor started (from ClApplicationService)");
                service.run_merge_processor_loop().await;
            });
        }
    }

    /// Main loop for the background merge processor.
    ///
    /// Continuously processes queue items until no active items remain.
    async fn run_merge_processor_loop(&self) {
        loop {
            match self.process_next_queue_item().await {
                Ok(processed) => {
                    if !processed {
                        // Check if there are active items
                        if let Ok(stats) =
                            self.storage().merge_queue_service.get_queue_stats().await
                        {
                            let has_active = stats.waiting_count > 0
                                || stats.testing_count > 0
                                || stats.merging_count > 0;

                            if !has_active {
                                // No active items, stop processor
                                self.storage().merge_queue_service.stop_processor();
                                tracing::info!("Merge queue processor stopped (no active items)");
                                break;
                            }
                        }
                        tokio::time::sleep(Duration::from_secs(Self::QUEUE_POLL_INTERVAL_SECS))
                            .await;
                    }
                }
                Err(e) => {
                    tracing::error!("Merge queue processor error: {}", e);
                    tokio::time::sleep(Duration::from_secs(Self::ERROR_BACKOFF_SECS)).await;
                }
            }
        }
    }

    /// Processes the next item in the merge queue.
    ///
    /// # Returns
    /// * `Ok(true)` - An item was processed (success or failure)
    /// * `Ok(false)` - No items to process
    /// * `Err(MegaError)` - System error occurred
    async fn process_next_queue_item(&self) -> Result<bool, MegaError> {
        let queue_service = &self.storage().merge_queue_service;

        // Get next waiting item from queue
        let next_item = queue_service.get_next_waiting_item().await?;

        if let Some(item) = next_item {
            let cl_link = item.cl_link.clone();

            // Update status to Testing
            let updated = queue_service
                .update_item_status(&cl_link, QueueStatusEnum::Testing)
                .await?;

            // Item was cancelled before we could start processing
            if !updated {
                return Ok(false);
            }

            // Execute the merge workflow
            match self.execute_merge_workflow(&cl_link).await {
                Ok(()) => {
                    // Success - status already updated to Merged in workflow
                    Ok(true)
                }
                Err((failure_type, message)) => {
                    if matches!(failure_type, QueueFailureTypeEnum::Conflict) {
                        // Conflict - move to tail of queue for retry
                        if let Err(e) = queue_service.move_item_to_tail(&cl_link).await {
                            tracing::warn!(
                                "Failed to move conflicting item {} to tail: {}",
                                cl_link,
                                e
                            );
                        }
                        Ok(false)
                    } else {
                        // Other failure - mark as failed
                        if let Err(e) = queue_service
                            .update_item_status_with_error(&cl_link, failure_type, message)
                            .await
                        {
                            tracing::error!(
                                "Failed to update item {} status to failed: {}",
                                cl_link,
                                e
                            );
                        }
                        Ok(true)
                    }
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Executes the complete merge workflow for a CL.
    ///
    /// Workflow steps:
    /// 1. Validate CL exists and is in valid status
    /// 2. Run tests (TODO: Buck2 integration)
    /// 3. Check for conflicts
    /// 4. Execute merge
    /// 5. Update statuses
    async fn execute_merge_workflow(
        &self,
        cl_link: &str,
    ) -> Result<(), (QueueFailureTypeEnum, String)> {
        let queue_service = &self.storage().merge_queue_service;

        // Step 1: Validate CL still exists and is not closed
        let cl = self
            .storage()
            .cl_service
            .cl_store()
            .get_cl(cl_link)
            .await
            .map_err(|e| {
                (
                    QueueFailureTypeEnum::SystemError,
                    format!("Failed to fetch CL: {}", e),
                )
            })?;

        let cl_model = match cl {
            Some(model) => {
                if model.status == MergeStatusEnum::Closed {
                    return Err((
                        QueueFailureTypeEnum::SystemError,
                        "CL has been closed, cannot merge".to_string(),
                    ));
                }
                if model.status == MergeStatusEnum::Draft {
                    return Err((
                        QueueFailureTypeEnum::SystemError,
                        "CL is in draft status, cannot merge".to_string(),
                    ));
                }
                model
            }
            None => {
                return Err((
                    QueueFailureTypeEnum::SystemError,
                    "CL no longer exists, cannot merge".to_string(),
                ));
            }
        };

        let updated = queue_service
            .update_item_status(cl_link, QueueStatusEnum::Merging)
            .await
            .map_err(|e| {
                (
                    QueueFailureTypeEnum::SystemError,
                    format!("Failed to update status to merging: {}", e),
                )
            })?;

        if !updated {
            return Err((
                QueueFailureTypeEnum::SystemError,
                "Item was cancelled".to_string(),
            ));
        }

        let merge_result = self.merge_cl("system", cl_model).await;

        if let Err(e) = merge_result {
            let message = e.to_string();
            let failure_type = if message.contains("conflict") || message.contains("Conflict") {
                QueueFailureTypeEnum::Conflict
            } else if message.contains("unmergeable") || message.contains("FAILED") {
                QueueFailureTypeEnum::SystemError
            } else {
                QueueFailureTypeEnum::MergeFailure
            };
            return Err((failure_type, format!("Merge failed: {message}")));
        }

        // Step 6: Update queue status to Merged
        queue_service
            .update_item_status(cl_link, QueueStatusEnum::Merged)
            .await
            .map_err(|e| {
                (
                    QueueFailureTypeEnum::SystemError,
                    format!("Failed to update status to merged: {}", e),
                )
            })?;

        Ok(())
    }
}
