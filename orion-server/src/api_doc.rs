use api_model::buck2::types::TaskPhase;
use utoipa::OpenApi;

use crate::api;

/// OpenAPI documentation configuration.
#[derive(OpenApi)]
#[openapi(
    paths(
        // Task domain
        api::task_handler,
        api::task_build_list_handler,
        api::task_output_handler,
        api::task_history_output_handler,
        api::tasks_handler,
        api::task_targets_handler,
        api::task_targets_summary_handler,
        api::task_retry_handler,
        api::task_get_handler,
        // Build domain
        api::build_retry_handler,
        api::build_event_get_handler,
        api::targets_get_handler,
        api::build_state_handler,
        api::build_logs_handler,
        api::latest_build_result_handler,
        // Worker domain
        api::get_orion_clients_info,
        api::get_orion_client_status_by_id,
        // Target status domain
        api::target_logs_handler,
        api::targets_status_handler,
        api::single_target_status_handle,
        // System domain
        api::health_check_handler
    ),
    components(
        schemas(
            crate::scheduler::BuildRequest,
            api_model::buck2::types::LogLinesResponse,
            api_model::buck2::types::TargetLogLinesResponse,
            api_model::buck2::types::LogErrorResponse,
            crate::model::task_status::TaskStatusEnum,
            crate::model::dto::BuildDTO,
            crate::model::dto::TargetDTO,
            crate::model::dto::TargetSummaryDTO,
            api_model::buck2::types::TargetLogQuery,
            api_model::buck2::types::LogReadMode,
            api_model::buck2::types::TaskHistoryQuery,
            crate::model::dto::TaskInfoDTO,
            crate::model::dto::OrionClientInfo,
            crate::model::dto::OrionClientStatus,
            crate::model::dto::CoreWorkerStatus,
            crate::model::dto::OrionClientQuery,
            crate::entity::targets::TargetState,
            TaskPhase,
            crate::model::dto::MessageResponse,
            crate::model::dto::BuildEventDTO,
            crate::model::dto::OrionTaskDTO,
            crate::model::dto::BuildTargetDTO,
            crate::model::dto::BuildEventState,
            api_model::buck2::types::TargetStatusResponse,
        )
    ),
    tags(
        (name = "Task", description = "Task lifecycle and task query endpoints"),
        (name = "Build", description = "Build dispatch/retry/state/log endpoints"),
        (name = "Worker", description = "Orion worker status and listing endpoints"),
        (name = "TargetStatus", description = "Target log and target status endpoints"),
        (name = "System", description = "Health and system-level endpoints")
    )
)]
pub struct ApiDoc;
