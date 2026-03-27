use uuid::Uuid;

use crate::model::target_state::TargetState;

/// Internal build-target representation used by scheduler/queue/repository code.
#[derive(Debug, Clone)]
pub(crate) struct BuildTargetStateDTO {
    pub(crate) id: Uuid,
    pub(crate) path: String,
    #[allow(dead_code)]
    pub(crate) state: TargetState,
}

pub(crate) mod target_build_status {
    use api_model::buck2::ws::WSTargetBuildStatusEvent;
    use callisto::sea_orm_active_enums::OrionTargetStatusEnum;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub(crate) struct NewTargetStatusInput {
        pub(crate) id: Uuid,
        pub(crate) task_id: Uuid,
        pub(crate) target_package: String,
        pub(crate) target_name: String,
        pub(crate) target_configuration: String,
        pub(crate) category: String,
        pub(crate) identifier: String,
        pub(crate) action: String,
        pub(crate) status: OrionTargetStatusEnum,
    }

    pub(crate) fn orion_target_status_from_ws(status: &str) -> OrionTargetStatusEnum {
        match status.trim().to_ascii_lowercase().as_str() {
            "pending" => OrionTargetStatusEnum::Pending,
            "running" => OrionTargetStatusEnum::Running,
            "success" | "succeeded" => OrionTargetStatusEnum::Success,
            "failed" => OrionTargetStatusEnum::Failed,
            _ => OrionTargetStatusEnum::Pending,
        }
    }

    /// API / WebSocket wire format for target build status (uppercase labels).
    pub(crate) fn orion_target_status_to_api_str(status: &OrionTargetStatusEnum) -> &'static str {
        match status {
            OrionTargetStatusEnum::Pending => "PENDING",
            OrionTargetStatusEnum::Running => "RUNNING",
            OrionTargetStatusEnum::Success => "SUCCESS",
            OrionTargetStatusEnum::Failed => "FAILED",
        }
    }

    impl NewTargetStatusInput {
        pub(crate) fn from_ws_event(task_id: Uuid, event: WSTargetBuildStatusEvent) -> Self {
            let status = orion_target_status_from_ws(&event.target.new_status);
            Self {
                id: Uuid::new_v4(),
                task_id,
                target_package: event.target.configured_target_package,
                target_name: event.target.configured_target_name,
                target_configuration: event.target.configured_target_configuration,
                category: event.target.category,
                identifier: event.target.identifier,
                action: event.target.action,
                status,
            }
        }
    }
}
