use common::config::Config;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{event::EventBase, event::EventType, queue::get_mq};

/// # Api Request Event
/// ---
/// This is a example event definition for using message queue.     \
///
/// Your customized event should implement `EventBase` trait.       \
/// Then the event can be put into message queue.                   \
/// The event `id` and `create_time` will be attached to your event
/// and then wrapped as a `Message`.                                \
/// You should also write some code in `mq::queue` to handle the event. (for now)
#[derive(Debug)]
pub struct ApiRequestEvent {
    pub api: ApiType,
    pub config: common::config::Config,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl std::fmt::Display for ApiRequestEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Api Request Event: {:?}", self.api)
    }
}

impl EventBase for ApiRequestEvent {}

impl ApiRequestEvent {
    // Create and enqueue this event.
    pub fn notify(api: ApiType, config: &Config) {
        get_mq().send(EventType::ApiRequest(ApiRequestEvent {
            api,
            config: config.clone(),
        }));
    }
}

// For storing the data into database.
impl Into<serde_json::Value> for ApiRequestEvent {
    fn into(self) -> serde_json::Value {
        json!({
            "api": self.api,
            "config": self.config
        })
    }
}

impl TryFrom<serde_json::Value> for ApiRequestEvent {
    type Error = crate::event::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let api: ApiType = serde_json::from_value(value["api"].clone())?;
        let config: common::config::Config = serde_json::from_value(value["config"].clone())?;

        Ok(ApiRequestEvent {
            api,
            config
        })
    }

}
