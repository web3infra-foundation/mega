use std::fmt::Display;

use api_request::ApiRequestEvent;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use github_webhook::GithubWebhookEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod api_request;
pub mod github_webhook;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    ApiRequest(ApiRequestEvent),
    GithubWebhook(GithubWebhookEvent),

    // Reserved
    ErrorEvent,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub(crate) id: i64,
    pub(crate) create_time: DateTime<Utc>,
    pub(crate) evt: EventType,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error converting from database")]
    MismatchedData(#[from] serde_json::error::Error),
}

#[async_trait]
pub trait EventBase:
    Send + Sync + std::fmt::Display + Into<serde_json::Value> + TryFrom<serde_json::Value>
{
    // defines the callback function for this event.
    async fn process(&self);
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl EventType {
    pub(crate) async fn process(&self) {
        match self {
            // I can't easily add a trait bound for the enum members,
            // so you have to manually add a process logic for your event here.
            EventType::ApiRequest(evt) => evt.process().await,
            // EventType::SomeOtherEvent(xxx) => xxx.process().await,
            EventType::GithubWebhook(evt) => evt.process().await,

            // This won't happen unless failed to load events from database.
            // And that's because of a event conversion error.
            // You should recheck yout conversion code logic.
            EventType::ErrorEvent => panic!("Got error event"),
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {}, Created at: {}", self.id, self.create_time)
    }
}

impl From<Message> for callisto::mq_storage::Model {
    fn from(val: Message) -> Self {
        use callisto::mq_storage::Model;

        let category = match val.evt {
            EventType::ApiRequest(_) => Some(String::from("ApiRequestEvent")),

            #[allow(unreachable_patterns)]
            _ => Some(String::from("Unknown")),
        };

        let content: Value = match val.evt {
            EventType::ApiRequest(evt) => evt.into(),

            #[allow(unreachable_patterns)]
            _ => Value::Null,
        };

        Model {
            id: val.id,
            category,
            create_time: val.create_time.naive_utc(),
            content: Some(content.to_string()),
        }
    }
}

impl From<callisto::mq_storage::Model> for Message {
    fn from(value: callisto::mq_storage::Model) -> Self {
        let id = value.id;
        let create_time = value.create_time.and_utc();
        let evt = match value.category.unwrap().as_str() {
            "ApiRequestEvent" => {
                if let Some(s) = value.content {
                    let evt = serde_json::from_str(&s).unwrap();
                    EventType::ApiRequest(evt)
                } else {
                    EventType::ErrorEvent
                }
            }

            _ => EventType::ErrorEvent,
        };

        Self {
            id,
            create_time,
            evt,
        }
    }
}
