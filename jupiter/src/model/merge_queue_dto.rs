use serde::{Deserialize, Serialize};

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
