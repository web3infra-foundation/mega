use api_model::buck2::types::TaskPhase;
use utoipa::OpenApi;

use crate::api;

/// OpenAPI documentation configuration.
#[derive(OpenApi)]
#[openapi(
    paths(
        // Task domain
        api::task_handler_v2,
        api::task_output_handler,
        api::task_history_output_handler,
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
            api_model::buck2::types::LogLinesResponse,
            api_model::buck2::types::TargetLogLinesResponse,
            api_model::buck2::types::LogErrorResponse,
            api_model::buck2::types::TargetLogQuery,
            api_model::buck2::types::LogReadMode,
            api_model::buck2::types::TaskHistoryQuery,
            crate::model::dto::OrionClientInfo,
            crate::model::dto::OrionClientStatus,
            crate::model::dto::CoreWorkerStatus,
            crate::model::dto::OrionClientQuery,
            crate::model::target_state::TargetState,
            TaskPhase,
            crate::model::dto::MessageResponse,
            crate::model::dto::BuildEventDTO,
            crate::model::dto::OrionTaskDTO,
            crate::model::dto::BuildTargetDTO,
            crate::model::dto::BuildStatus,
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
