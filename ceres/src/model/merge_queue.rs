use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// CL queue status for API
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum QueueStatus {
    Waiting,
    Testing,
    Merging,
    Merged,
    Failed,
}

/// Failure type for API
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum FailureType {
    TestFailure,
    BuildFailure,
    Conflict,
    MergeFailure,
    SystemError,
    Timeout,
}

/// Error details for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueueError {
    pub failure_type: FailureType,
    pub message: String,
    pub occurred_at: String,
}

/// Queue item for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueueItem {
    pub cl_link: String,
    pub status: QueueStatus,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
    pub retry_count: i32,
    pub error: Option<QueueError>,
}

/// Queue statistics for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueueStats {
    pub total_items: usize,
    pub waiting_count: usize,
    pub testing_count: usize,
    pub merging_count: usize,
    pub failed_count: usize,
    pub merged_count: usize,
}

/// Add CL to queue request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddToQueueRequest {
    pub cl_link: String,
}

/// Add CL to queue response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddToQueueResponse {
    pub success: bool,
    pub position: i32,
    pub message: String,
}

/// Queue list response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueueListResponse {
    pub items: Vec<QueueItem>,
    pub total_count: usize,
}

/// Queue status check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueueStatusResponse {
    pub in_queue: bool,
    pub item: Option<QueueItem>,
}

/// Queue statistics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueueStatsResponse {
    pub stats: QueueStats,
}

impl From<jupiter::model::merge_queue::QueueStatus> for QueueStatus {
    fn from(status: jupiter::model::merge_queue::QueueStatus) -> Self {
        match status {
            jupiter::model::merge_queue::QueueStatus::Waiting => QueueStatus::Waiting,
            jupiter::model::merge_queue::QueueStatus::Testing => QueueStatus::Testing,
            jupiter::model::merge_queue::QueueStatus::Merging => QueueStatus::Merging,
            jupiter::model::merge_queue::QueueStatus::Merged => QueueStatus::Merged,
            jupiter::model::merge_queue::QueueStatus::Failed => QueueStatus::Failed,
        }
    }
}

impl From<jupiter::model::merge_queue::FailureType> for FailureType {
    fn from(failure_type: jupiter::model::merge_queue::FailureType) -> Self {
        match failure_type {
            jupiter::model::merge_queue::FailureType::TestFailure => FailureType::TestFailure,
            jupiter::model::merge_queue::FailureType::BuildFailure => FailureType::BuildFailure,
            jupiter::model::merge_queue::FailureType::Conflict => FailureType::Conflict,
            jupiter::model::merge_queue::FailureType::MergeFailure => FailureType::MergeFailure,
            jupiter::model::merge_queue::FailureType::SystemError => FailureType::SystemError,
            jupiter::model::merge_queue::FailureType::Timeout => FailureType::Timeout,
        }
    }
}

impl From<jupiter::model::merge_queue::QueueError> for QueueError {
    fn from(error: jupiter::model::merge_queue::QueueError) -> Self {
        QueueError {
            failure_type: error.failure_type.into(),
            message: error.message,
            occurred_at: error.occurred_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

impl From<jupiter::model::merge_queue::QueueItem> for QueueItem {
    fn from(item: jupiter::model::merge_queue::QueueItem) -> Self {
        QueueItem {
            cl_link: item.cl_link,
            status: item.status.into(),
            position: item.position,
            created_at: item.added_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: item
                .last_retry_at
                .unwrap_or(item.added_at)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            retry_count: item.retry_count,
            error: item.error_details.map(|e| e.into()),
        }
    }
}

impl From<jupiter::model::merge_queue::QueueStats> for QueueStats {
    fn from(stats: jupiter::model::merge_queue::QueueStats) -> Self {
        QueueStats {
            total_items: stats.total_items,
            waiting_count: stats.waiting_count,
            testing_count: stats.testing_count,
            merging_count: stats.merging_count,
            failed_count: stats.failed_count,
            merged_count: stats.merged_count,
        }
    }
}

impl From<Vec<jupiter::model::merge_queue::QueueItem>> for QueueListResponse {
    fn from(items: Vec<jupiter::model::merge_queue::QueueItem>) -> Self {
        let total_count = items.len();
        let api_items: Vec<QueueItem> = items.into_iter().map(|item| item.into()).collect();

        QueueListResponse {
            items: api_items,
            total_count,
        }
    }
}

impl From<Option<jupiter::model::merge_queue::QueueItem>> for QueueStatusResponse {
    fn from(item: Option<jupiter::model::merge_queue::QueueItem>) -> Self {
        match item {
            Some(queue_item) => QueueStatusResponse {
                in_queue: true,
                item: Some(queue_item.into()),
            },
            None => QueueStatusResponse {
                in_queue: false,
                item: None,
            },
        }
    }
}

impl From<jupiter::model::merge_queue::QueueStats> for QueueStatsResponse {
    fn from(stats: jupiter::model::merge_queue::QueueStats) -> Self {
        QueueStatsResponse {
            stats: stats.into(),
        }
    }
}
