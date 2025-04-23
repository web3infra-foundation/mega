use async_trait::async_trait;
use common::config::Config;
use serde::{Deserialize, Serialize};

use crate::{event::EventBase, event::EventType, queue::get_mq};

/// # Api Request Event
///
/// This is a example event definition for using message queue.
///
/// Your customized event should implement `EventBase` trait.
/// Then the event can be wrapped and put into message queue.
/// The message `id` and `create_time` will be attached to your
/// event and then wrapped as a `Message`.
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
impl From<ApiRequestEvent> for serde_json::Value {
    fn from(value: ApiRequestEvent) -> Self {
        serde_json::to_value(value).unwrap()
    }
}

impl TryFrom<serde_json::Value> for ApiRequestEvent {
    type Error = crate::event::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let res: ApiRequestEvent = serde_json::from_value(value)?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiRequestEvent, ApiType};
    use common::config::Config;
    use serde_json::Value;

    const SER: &str = r#"{"api":"Blob","config":{"base_dir":"","database":{"db_path":"/tmp/.mega/mega.db","db_type":"sqlite","db_url":"postgres://mega:mega@localhost:5432/mega","max_connection":32,"min_connection":16,"sqlx_logging":false},"lfs":{"enable_split":true,"split_size":1073741824},"log":{"level":"info","log_path":"/tmp/.mega/logs","print_std":true},"monorepo":{"import_dir":"/third-part"},"oauth":{"github_client_id":"","github_client_secret":""},"pack":{"channel_message_size":1000000,"clean_cache_after_decode":true,"pack_decode_cache_path":"/tmp/.mega/cache","pack_decode_mem_size":4,pack_decode_disk_size:"20%"},"ssh":{"ssh_key_path":"/tmp/.mega/ssh"},"storage":{"big_obj_threshold":1024,"lfs_obj_local_path":"/tmp/.mega/lfs","obs_access_key":"","obs_endpoint":"https://obs.cn-east-3.myhuaweicloud.com","obs_region":"cn-east-3","obs_secret_key":"","raw_obj_local_path":"/tmp/.mega/objects","raw_obj_storage_type":"LOCAL"},"ztm":{"agent":"127.0.0.1:7777","ca":"127.0.0.1:9999","hub":"127.0.0.1:8888"}}}"#;

    #[test]
    fn test_conversion() {
        let evt = ApiRequestEvent {
            api: ApiType::Blob,
            config: Config::default(),
        };

        // Convert into value
        let serialized: Value = Value::from(evt);
        assert_eq!(serialized.to_string().as_str(), SER);

        // Convert from value
        let _ = ApiRequestEvent::try_from(serialized).unwrap();
    }
}
