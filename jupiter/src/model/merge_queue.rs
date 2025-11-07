use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// CL status in merge queue
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum QueueStatus {
    #[serde(rename = "waiting")]
    #[default]
    Waiting,
    #[serde(rename = "testing")]
    Testing,
    #[serde(rename = "merging")]
    Merging,
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "failed")]
    Failed,
}

impl QueueStatus {
    /// Validates status transition
    pub fn can_transition_to(&self, target: &QueueStatus) -> bool {
        match (self, target) {
            (QueueStatus::Waiting, QueueStatus::Testing) => true,
            (QueueStatus::Waiting, QueueStatus::Failed) => true,
            (QueueStatus::Testing, QueueStatus::Waiting) => true,
            (QueueStatus::Testing, QueueStatus::Merging) => true,
            (QueueStatus::Testing, QueueStatus::Failed) => true,
            (QueueStatus::Merging, QueueStatus::Merged) => true,
            (QueueStatus::Merging, QueueStatus::Failed) => true,
            (QueueStatus::Failed, QueueStatus::Waiting) => true,
            (a, b) if a == b => true,
            _ => false,
        }
    }

    /// Checks if status is failed
    pub fn is_failed(&self) -> bool {
        matches!(self, QueueStatus::Failed)
    }
}

/// Failure types during merge queue processing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FailureType {
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "test_failure")]
    TestFailure,
    #[serde(rename = "build_failure")]
    BuildFailure,
    #[serde(rename = "merge_failure")]
    MergeFailure,
    #[serde(rename = "system_error")]
    SystemError,
    #[serde(rename = "timeout")]
    Timeout,
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureType::Conflict => write!(f, "Conflict"),
            FailureType::TestFailure => write!(f, "Test Failure"),
            FailureType::BuildFailure => write!(f, "Build Failure"),
            FailureType::MergeFailure => write!(f, "Merge Failure"),
            FailureType::SystemError => write!(f, "System Error"),
            FailureType::Timeout => write!(f, "Timeout"),
        }
    }
}

/// Error details for failed queue items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueError {
    pub failure_type: FailureType,
    pub message: String,
    pub occurred_at: DateTime<Utc>,
}

impl QueueError {
    /// Creates new queue error
    pub fn new(failure_type: FailureType, message: String) -> Self {
        Self {
            failure_type,
            message,
            occurred_at: Utc::now(),
        }
    }
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.failure_type, self.message)
    }
}

/// Single item in merge queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub cl_link: String,
    pub position: i32,
    pub status: QueueStatus,
    pub added_at: DateTime<Utc>,
    pub retry_count: i32,
    pub last_retry_at: Option<DateTime<Utc>>,
    pub error_details: Option<QueueError>,
}

impl QueueItem {
    /// Creates new waiting queue item
    pub fn new(cl_link: String, position: i32) -> Self {
        Self {
            cl_link,
            position,
            status: QueueStatus::Waiting,
            added_at: Utc::now(),
            retry_count: 0,
            last_retry_at: None,
            error_details: None,
        }
    }

    /// Updates status with validation
    pub fn update_status(&mut self, status: QueueStatus) -> Result<(), String> {
        if !self.status.can_transition_to(&status) {
            return Err(format!(
                "Invalid status transition from {:?} to {:?}",
                self.status, status
            ));
        }

        self.status = status;

        if !status.is_failed() {
            self.error_details = None;
        }

        Ok(())
    }

    /// Updates status with error details
    pub fn update_status_with_error(
        &mut self,
        status: QueueStatus,
        error: QueueError,
    ) -> Result<(), String> {
        self.update_status(status)?;
        self.error_details = Some(error);
        Ok(())
    }

    /// Increments retry counter
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.last_retry_at = Some(Utc::now());
    }

    /// Checks if item can be retried
    pub fn can_retry(&self, max_retries: i32) -> bool {
        self.retry_count < max_retries && self.status == QueueStatus::Failed
    }
}

/// Queue data container (used for in-memory operations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueData {
    pub items: Vec<QueueItem>,
}

impl QueueData {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds item with timestamp-based position
    pub fn add_item(&mut self, cl_link: String) -> i32 {
        let position = chrono::Utc::now().timestamp() as i32;
        let item = QueueItem::new(cl_link, position);
        self.items.push(item);
        position
    }

    pub fn remove_item(&mut self, cl_link: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|item| item.cl_link == cl_link) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }
}

impl Default for QueueData {
    fn default() -> Self {
        Self::new()
    }
}

/// Queue statistics
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct QueueStats {
    pub total_items: usize,
    pub waiting_count: usize,
    pub testing_count: usize,
    pub merging_count: usize,
    pub merged_count: usize,
    pub failed_count: usize,
}
