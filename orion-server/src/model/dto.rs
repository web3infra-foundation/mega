use api_model::buck2::types::TaskPhase;
use chrono::Utc;
use sea_orm::prelude::DateTimeUtc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;

use crate::{
    entity::{builds, targets},
    model::task_status::TaskStatusEnum,
    scheduler::{WorkerInfo, WorkerStatus},
};

/// Data transfer object for build information in API responses.
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct BuildDTO {
    pub id: String,
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    pub exit_code: Option<i32>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub repo: String,
    pub target: String,
    pub args: Option<Value>,
    pub output_file: String,
    pub created_at: String,
    pub retry_count: i32,
    pub status: TaskStatusEnum,
    pub cause_by: Option<String>,
}

impl BuildDTO {
    pub fn from_model(
        model: builds::Model,
        target: Option<&targets::Model>,
        status: TaskStatusEnum,
    ) -> Self {
        let target_path = target.map(|t| t.target_path.clone()).unwrap_or_default();
        Self {
            id: model.id.to_string(),
            task_id: model.task_id.to_string(),
            target_id: target.map(|t| t.id.to_string()),
            exit_code: model.exit_code,
            start_at: model.start_at.with_timezone(&Utc).to_rfc3339(),
            end_at: model.end_at.map(|dt| dt.with_timezone(&Utc).to_rfc3339()),
            repo: model.repo,
            target: target_path,
            args: model.args.map(|v| json!(v)),
            output_file: model.output_file,
            created_at: model.created_at.with_timezone(&Utc).to_rfc3339(),
            retry_count: model.retry_count,
            status,
            cause_by: None,
        }
    }

    pub fn determine_status(model: &builds::Model, is_active: bool) -> TaskStatusEnum {
        if is_active {
            TaskStatusEnum::Building
        } else if model.end_at.is_none() {
            TaskStatusEnum::Pending
        } else if model.exit_code.is_none() {
            TaskStatusEnum::Interrupted
        } else if model.exit_code == Some(0) {
            TaskStatusEnum::Completed
        } else {
            TaskStatusEnum::Failed
        }
    }
}

pub type TargetDTO = targets::TargetWithBuilds<BuildDTO>;

#[derive(Debug, Serialize, ToSchema)]
pub struct TaskInfoDTO {
    pub task_id: String,
    pub cl_id: i64,
    pub task_name: Option<String>,
    pub template: Option<serde_json::Value>,
    pub created_at: String,
    pub build_list: Vec<BuildDTO>,
    pub targets: Vec<TargetDTO>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TargetSummaryDTO {
    pub task_id: String,
    pub pending: u64,
    pub building: u64,
    pub completed: u64,
    pub failed: u64,
    pub interrupted: u64,
    pub uninitialized: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrionClientInfo {
    pub client_id: String,
    pub hostname: String,
    pub orion_version: String,
    #[schema(value_type = String, format = "date-time")]
    pub start_time: DateTimeUtc,
    #[schema(value_type = String, format = "date-time")]
    pub last_heartbeat: DateTimeUtc,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub enum CoreWorkerStatus {
    Idle,
    Busy,
    Error,
    Lost,
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct OrionClientQuery {
    pub hostname: Option<String>,
    pub status: Option<CoreWorkerStatus>,
    pub phase: Option<TaskPhase>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrionClientStatus {
    pub core_status: CoreWorkerStatus,
    pub phase: Option<TaskPhase>,
    pub error_message: Option<String>,
}

impl OrionClientStatus {
    pub fn from_worker_status(worker: &WorkerInfo) -> Self {
        match &worker.status {
            WorkerStatus::Idle => Self {
                core_status: CoreWorkerStatus::Idle,
                phase: None,
                error_message: None,
            },
            WorkerStatus::Busy { phase, .. } => Self {
                core_status: CoreWorkerStatus::Busy,
                phase: phase.clone(),
                error_message: None,
            },
            WorkerStatus::Error(msg) => Self {
                core_status: CoreWorkerStatus::Error,
                phase: None,
                error_message: Some(msg.clone()),
            },
            WorkerStatus::Lost => Self {
                core_status: CoreWorkerStatus::Lost,
                phase: None,
                error_message: None,
            },
        }
    }
}

#[derive(ToSchema, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(ToSchema, Serialize)]
pub struct BuildTargetDTO {
    pub id: String,
    pub task_id: String,
    pub path: String,
    pub latest_state: String,
}

#[derive(ToSchema, Serialize)]
pub struct BuildEventDTO {
    pub id: String,
    pub task_id: String,
    pub retry_count: i32,
    pub exit_code: Option<i32>,
    pub log: Option<String>,
    pub log_output_file: String,
    pub start_at: String,
    pub end_at: Option<String>,
}

impl From<&callisto::build_events::Model> for BuildEventDTO {
    fn from(model: &callisto::build_events::Model) -> Self {
        Self {
            id: model.id.to_string(),
            task_id: model.task_id.to_string(),
            retry_count: model.retry_count,
            exit_code: model.exit_code,
            log: model.log.clone(),
            log_output_file: model.log_output_file.clone(),
            start_at: model.start_at.to_string(),
            end_at: model.end_at.map(|dt| dt.with_timezone(&Utc).to_string()),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct OrionTaskDTO {
    pub id: String,
    pub changes: Value,
    pub repo_name: String,
    pub cl: String,
    pub created_at: String,
}

impl From<&callisto::orion_tasks::Model> for OrionTaskDTO {
    fn from(model: &callisto::orion_tasks::Model) -> Self {
        Self {
            id: model.id.to_string(),
            changes: model.changes.clone(),
            repo_name: model.repo_name.clone(),
            cl: model.cl.clone(),
            created_at: model.created_at.with_timezone(&Utc).to_string(),
        }
    }
}

#[derive(ToSchema, Serialize)]
pub enum BuildEventState {
    #[allow(dead_code)]
    Pending,
    Running,
    Success,
    Failure,
}
