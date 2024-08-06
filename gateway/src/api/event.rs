use axum::extract::State;

use crate::api::ApiServiceState;

use mq::{event::EventBase, queue::get_mq};

#[derive(Debug)]
pub enum ApiType {
    // Common Api enum for api_routers
    CreateFile,
    LastestCommit,
    CommitInfo,
    TreeInfo,
    Blob,
    Publish,

    // Merge Api enum for mr_routers
    MergeRequest,
    MergeDone,
    MergeList,
    MergeDetail,
    MergeFiles,
}

pub struct ApiRequestEvent {
    pub api: ApiType,
    pub state: State<ApiServiceState>,
}

impl std::fmt::Display for ApiRequestEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Api Request Event: {:?}", self.api)
    }
}

impl EventBase for ApiRequestEvent {

}

impl ApiRequestEvent {
    // Create and enqueue this event.
    pub fn notify(api: ApiType, state: &State<ApiServiceState>) {
        get_mq().send(Box::new(ApiRequestEvent {
            api,
            state: state.clone()
        }));
    }
}
