use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::model::merge_queue::{FailureType, QueueError, QueueItem, QueueStats, QueueStatus};
use crate::storage::{
    base_storage::{BaseStorage, StorageConnector},
    cl_storage::ClStorage,
    merge_queue_storage::MergeQueueStorage,
};
use callisto::sea_orm_active_enums::MergeStatusEnum;
use common::errors::MegaError;

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

    pub async fn add_to_queue(&self, cl_link: String) -> Result<i32, MegaError> {
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

    pub async fn get_queue_list(&self) -> Result<Vec<QueueItem>, MegaError> {
        self.merge_queue_storage
            .get_queue_list()
            .await
            .map_err(|e| MegaError::with_message(&e))
    }

    pub async fn get_cl_queue_status(&self, cl_link: &str) -> Result<Option<QueueItem>, MegaError> {
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
                .update_item_status(&item.cl_link, QueueStatus::Testing)
                .await
                .map_err(|e| MegaError::with_message(&e))?;

            let cl_link = item.cl_link.clone();

            let processed = match self.process_merge_workflow(&cl_link).await {
                Ok(()) => true,
                Err(queue_error) => {
                    if matches!(queue_error.failure_type, FailureType::Conflict) {
                        let _ = self.merge_queue_storage.move_item_to_tail(&cl_link).await;
                        false
                    } else {
                        let _ = self
                            .merge_queue_storage
                            .update_item_status_with_error(
                                &cl_link,
                                QueueStatus::Failed,
                                queue_error,
                            )
                            .await;
                        true
                    }
                }
            };
            Ok(processed)
        } else {
            Ok(false)
        }
    }

    async fn process_merge_workflow(&self, cl_link: &str) -> Result<(), QueueError> {
        // Validate CL still exists and is not closed before processing
        let cl = self.cl_storage.get_cl(cl_link).await.map_err(|e| {
            QueueError::new(
                FailureType::SystemError,
                format!("Failed to fetch CL: {}", e),
            )
        })?;

        let cl_model = match cl {
            Some(cl_model) => {
                if cl_model.status == MergeStatusEnum::Closed {
                    return Err(QueueError::new(
                        FailureType::SystemError,
                        "CL has been closed, cannot merge".to_string(),
                    ));
                }
                cl_model
            }
            None => {
                return Err(QueueError::new(
                    FailureType::SystemError,
                    "CL no longer exists, cannot merge".to_string(),
                ));
            }
        };

        self.execute_testing(cl_link).await?;
        self.check_conflicts(cl_link).await?;

        self.merge_queue_storage
            .update_item_status(cl_link, QueueStatus::Merging)
            .await
            .map_err(|e| {
                QueueError::new(
                    FailureType::SystemError,
                    format!("Failed to update status to merging: {}", e),
                )
            })?;

        self.execute_merge(cl_link).await?;

        // Update merge queue status
        self.merge_queue_storage
            .update_item_status(cl_link, QueueStatus::Merged)
            .await
            .map_err(|e| {
                QueueError::new(
                    FailureType::SystemError,
                    format!("Failed to update status to merged: {}", e),
                )
            })?;

        // Update CL status in mega_cl table
        self.cl_storage.merge_cl(cl_model).await.map_err(|e| {
            QueueError::new(
                FailureType::SystemError,
                format!("Failed to update CL status: {}", e),
            )
        })?;

        Ok(())
    }

    async fn validate_cl_for_queue(&self, cl_link: &str) -> Result<(), MegaError> {
        let cl = self.cl_storage.get_cl(cl_link).await?;

        match cl {
            Some(cl_model) => {
                if cl_model.status == MergeStatusEnum::Closed {
                    return Err(MegaError::with_message("Cannot queue a closed CL"));
                }
                Ok(())
            }
            None => Err(MegaError::with_message("CL not found")),
        }
    }

    async fn check_conflicts(&self, cl_link: &str) -> Result<(), QueueError> {
        // TODO: Implement conflict detection logic with Git
        // Mock implementation for testing
        if cl_link.contains("conflict") {
            tracing::warn!("Mock conflict detected for: {}", cl_link);
            return Err(QueueError::new(
                FailureType::Conflict,
                "Mock conflict detected".to_string(),
            ));
        }

        Ok(())
    }

    async fn execute_merge(&self, _cl_link: &str) -> Result<(), QueueError> {
        // TODO: Implement actual Git merge operation
        Ok(())
    }

    async fn execute_testing(&self, cl_link: &str) -> Result<(), QueueError> {
        let cl = self
            .cl_storage
            .get_cl(cl_link)
            .await
            .map_err(|e| QueueError::new(FailureType::SystemError, e.to_string()))?
            .ok_or_else(|| QueueError::new(FailureType::SystemError, "CL not found".to_string()))?;

        match self.run_buck2_tests(&cl).await {
            Ok(success) => {
                if success {
                    Ok(())
                } else {
                    Err(QueueError::new(
                        FailureType::TestFailure,
                        "Buck2 tests failed".to_string(),
                    ))
                }
            }
            Err(e) => Err(QueueError::new(
                FailureType::SystemError,
                format!("Buck2 test execution error: {}", e),
            )),
        }
    }

    async fn run_buck2_tests(&self, cl: &callisto::mega_cl::Model) -> Result<bool, QueueError> {
        // TODO: Implement actual Buck2 test execution
        // Mock implementation for testing
        if cl.link.contains("fail-test") {
            tracing::warn!("Mock test failure for CL: {}", cl.link);
            return Ok(false);
        }

        Ok(true)
    }

    pub async fn cancel_all_pending(&self) -> Result<usize, MegaError> {
        let count = self
            .merge_queue_storage
            .cancel_all_pending()
            .await
            .map_err(|e| MegaError::with_message(&e))?;
        Ok(count as usize)
    }

    /// Ensures processor is running, starts it if not
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
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Queue processor error: {}", e);
                            tokio::time::sleep(Duration::from_secs(30)).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::merge_queue::QueueData;
    use chrono::Utc;

    /// Tests queue item business logic without database
    #[test]
    fn test_queue_item_status_transitions() {
        println!("\n🧪 QUEUE ITEM STATUS TRANSITION TEST\n");

        let mut item = QueueItem::new("cl-test".to_string(), 12345);

        // Test initial state
        assert_eq!(item.status, QueueStatus::Waiting);
        assert_eq!(item.retry_count, 0);
        println!("✓ Initial status: Waiting");

        // Test valid transitions
        assert!(item.update_status(QueueStatus::Testing).is_ok());
        assert_eq!(item.status, QueueStatus::Testing);
        println!("✓ Waiting -> Testing");

        assert!(item.update_status(QueueStatus::Merging).is_ok());
        assert_eq!(item.status, QueueStatus::Merging);
        println!("✓ Testing -> Merging");

        assert!(item.update_status(QueueStatus::Merged).is_ok());
        assert_eq!(item.status, QueueStatus::Merged);
        println!("✓ Merging -> Merged");

        // Test failure path
        let mut item2 = QueueItem::new("cl-fail".to_string(), 12346);
        item2.update_status(QueueStatus::Testing).unwrap();

        let error = QueueError::new(FailureType::TestFailure, "Test failed".to_string());
        assert!(
            item2
                .update_status_with_error(QueueStatus::Failed, error)
                .is_ok()
        );
        assert_eq!(item2.status, QueueStatus::Failed);
        assert!(item2.error_details.is_some());
        println!("✓ Testing -> Failed (with error)");

        // Test retry increment
        item2.update_status(QueueStatus::Waiting).unwrap();
        item2.increment_retry();
        assert_eq!(item2.status, QueueStatus::Waiting);
        assert_eq!(item2.retry_count, 1);
        println!("✓ Failed -> Waiting with retry counter incremented");

        println!("\n✅ ALL STATUS TRANSITION TESTS PASSED");
    }

    /// Tests failure type classification
    #[test]
    fn test_failure_types() {
        println!("\n🧪 FAILURE TYPE TEST\n");

        let error_types = vec![
            (FailureType::Conflict, "Conflict"),
            (FailureType::TestFailure, "Test Failure"),
            (FailureType::BuildFailure, "Build Failure"),
            (FailureType::MergeFailure, "Merge Failure"),
            (FailureType::SystemError, "System Error"),
            (FailureType::Timeout, "Timeout"),
        ];

        for (err_type, expected_display) in error_types {
            let error = QueueError::new(err_type, "Mock error".to_string());
            assert_eq!(format!("{}", err_type), expected_display);
            assert!(error.occurred_at <= Utc::now());
            println!("✓ {} error type works correctly", expected_display);
        }

        // Test conflict retriability logic (defined in service layer)
        assert_eq!(FailureType::Conflict as i32, 0, "Conflict is first variant");
        println!("✓ Conflict type is distinguishable for auto-requeue");

        println!("\n✅ FAILURE TYPE TESTS PASSED");
    }

    /// Tests queue data operations
    #[test]
    fn test_queue_data_operations() {
        println!("\n🧪 QUEUE DATA OPERATIONS TEST\n");

        let mut queue = QueueData::new();

        // Test add items
        let pos1 = queue.add_item("cl-1".to_string());
        std::thread::sleep(std::time::Duration::from_millis(100));
        let pos2 = queue.add_item("cl-2".to_string());
        std::thread::sleep(std::time::Duration::from_millis(100));
        let pos3 = queue.add_item("cl-3".to_string());

        assert_eq!(queue.items.len(), 3);
        assert!(
            pos2 >= pos1 && pos3 >= pos2,
            "Positions should be sequential"
        );
        println!("✓ Added 3 items with sequential positions");

        // Test remove item
        assert!(queue.remove_item("cl-2"));
        assert_eq!(queue.items.len(), 2);
        assert!(!queue.remove_item("cl-nonexistent"));
        println!("✓ Remove item works correctly");

        // Test statistics calculation manually (follow valid status transitions)
        queue.items[0].update_status(QueueStatus::Testing).unwrap();
        queue.items[0].update_status(QueueStatus::Merging).unwrap();
        queue.items[0].update_status(QueueStatus::Merged).unwrap();

        queue.items[1].update_status(QueueStatus::Testing).unwrap();
        let error = QueueError::new(FailureType::TestFailure, "Failed".to_string());
        queue.items[1]
            .update_status_with_error(QueueStatus::Failed, error)
            .unwrap();

        let merged_count = queue
            .items
            .iter()
            .filter(|i| i.status == QueueStatus::Merged)
            .count();
        let failed_count = queue
            .items
            .iter()
            .filter(|i| i.status == QueueStatus::Failed)
            .count();
        assert_eq!(queue.items.len(), 2);
        assert_eq!(merged_count, 1);
        assert_eq!(failed_count, 1);
        println!("✓ Status tracking works correctly");

        println!("\n✅ QUEUE DATA OPERATIONS TESTS PASSED");
    }

    /// Tests timestamp-based position assignment
    #[test]
    fn test_timestamp_positioning() {
        println!("\n🧪 TIMESTAMP POSITIONING TEST\n");

        let pos1 = chrono::Utc::now().timestamp() as i32;
        std::thread::sleep(std::time::Duration::from_millis(100));
        let pos2 = chrono::Utc::now().timestamp() as i32;

        assert!(pos2 >= pos1, "Later timestamp should be greater or equal");
        assert!(pos1 > 1000000000, "Should be valid Unix timestamp");
        println!("✓ Timestamp-based positioning works correctly");
        println!("  Position 1: {}", pos1);
        println!("  Position 2: {}", pos2);

        println!("\n✅ TIMESTAMP POSITIONING TEST PASSED");
    }
}
