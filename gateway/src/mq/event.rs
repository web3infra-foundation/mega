use axum::extract::State;

use crate::api::ApiServiceState;

use super::queue::get_mq;

pub(crate) type Message = Event;

pub enum Event {
    Api(ApiRequestEvent),
}

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

// pub trait EventBase: Send + Sync {
//     type Type: Into<EventType>;
//     fn event_type(&self) -> Self::Type;

//     // async fn process(&self);
// }

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            crate::mq::event::Event::Api(evt) => write!(f, "{}", evt),

            #[allow(unreachable_patterns)]
            _ => write!(f, "Unknown Event Type")
        }
    }
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

impl ApiRequestEvent {
    // Create and enqueue this event.
    pub fn notice(api: ApiType, state: &State<ApiServiceState>) {
        get_mq().send(Event::Api(ApiRequestEvent {
            api,
            state: state.clone()
        }));
    }
}
