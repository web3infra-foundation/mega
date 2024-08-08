use common::config::Config;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::{event::EventBase, event::EventType, queue::get_mq};

/// # Api Request Event
/// ---
/// This is a example event definition for using message queue.     \
///
/// Your customized event should implement `EventBase` trait.       \
/// Then the event can be wrapped and put into message queue.       \
/// The message `id` and `create_time` will be attached to your event
/// and then wrapped as a `Message`.                                \
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[async_trait]
impl EventBase for ApiRequestEvent {
    async fn process(&self) {
        tracing::info!("Handling Api Request event: [{}]", &self);
    }
}

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
        serde_json::to_value(self).unwrap()
    }
}

impl TryFrom<serde_json::Value> for ApiRequestEvent {
    type Error = crate::event::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let res: ApiRequestEvent = serde_json::from_value(value)?;
        Ok(res)
    }

}
