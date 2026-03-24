use serde::Serialize;
use utoipa::ToSchema;

/// Enumeration of possible task statuses
#[derive(Debug, Serialize, Default, ToSchema, Clone)]
pub enum TaskStatusEnum {
    /// Task is queued and waiting to be assigned to a worker
    Pending,
    Building,
    Interrupted, // Task was interrupted, exit code is None
    Failed,
    Completed,
    #[default]
    NotFound,
}
