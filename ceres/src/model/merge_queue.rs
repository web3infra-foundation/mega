use callisto::{
    merge_queue::Model,
    sea_orm_active_enums::{QueueFailureTypeEnum, QueueStatusEnum},
};
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
    pub position: i64,
    pub display_position: Option<usize>,
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
    pub position: i64,
    pub display_position: Option<usize>,
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

impl From<QueueStatusEnum> for QueueStatus {
    fn from(status: QueueStatusEnum) -> Self {
        match status {
            QueueStatusEnum::Waiting => QueueStatus::Waiting,
            QueueStatusEnum::Testing => QueueStatus::Testing,
            QueueStatusEnum::Merging => QueueStatus::Merging,
            QueueStatusEnum::Merged => QueueStatus::Merged,
            QueueStatusEnum::Failed => QueueStatus::Failed,
        }
    }
}

impl From<QueueFailureTypeEnum> for FailureType {
    fn from(failure_type: QueueFailureTypeEnum) -> Self {
        match failure_type {
            QueueFailureTypeEnum::TestFailure => FailureType::TestFailure,
            QueueFailureTypeEnum::BuildFailure => FailureType::BuildFailure,
            QueueFailureTypeEnum::Conflict => FailureType::Conflict,
            QueueFailureTypeEnum::MergeFailure => FailureType::MergeFailure,
            QueueFailureTypeEnum::SystemError => FailureType::SystemError,
            QueueFailureTypeEnum::Timeout => FailureType::Timeout,
        }
    }
}

impl From<Model> for QueueItem {
    fn from(item: Model) -> Self {
        let error = item.failure_type.map(|ft| {
            let occurred_at_local = item
                .updated_at
                .and_utc()
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();

            QueueError {
                failure_type: ft.into(),
                message: item.error_message.unwrap_or_default(),
                occurred_at: occurred_at_local,
            }
        });

        QueueItem {
            cl_link: item.cl_link,
            status: item.status.into(),
            position: item.position,
            display_position: None,
            created_at: item
                .created_at
                .and_utc()
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            updated_at: item
                .updated_at
                .and_utc()
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            retry_count: item.retry_count,
            error,
        }
    }
}

impl From<jupiter::model::merge_queue_dto::QueueStats> for QueueStats {
    fn from(stats: jupiter::model::merge_queue_dto::QueueStats) -> Self {
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

impl From<Vec<Model>> for QueueListResponse {
    fn from(items: Vec<Model>) -> Self {
        let total_count = items.len();
        let mut api_items: Vec<QueueItem> = items.into_iter().map(|item| item.into()).collect();
        for (idx, item) in api_items.iter_mut().enumerate() {
            item.display_position = Some(idx + 1);
        }

        QueueListResponse {
            items: api_items,
            total_count,
        }
    }
}

impl From<Option<Model>> for QueueStatusResponse {
    fn from(item: Option<Model>) -> Self {
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

impl From<jupiter::model::merge_queue_dto::QueueStats> for QueueStatsResponse {
    fn from(stats: jupiter::model::merge_queue_dto::QueueStats) -> Self {
        let ceres_stats: QueueStats = stats.into();

        ceres_stats.into()
    }
}

impl From<QueueStats> for QueueStatsResponse {
    fn from(stats: QueueStats) -> Self {
        QueueStatsResponse { stats }
    }
}
