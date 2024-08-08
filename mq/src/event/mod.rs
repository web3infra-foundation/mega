use std::fmt::Display;

use api_request::ApiRequestEvent;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod api_request;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    ApiRequest(ApiRequestEvent),

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

pub trait EventBase:
    Send + Sync + std::fmt::Display + Into<serde_json::Value> + TryFrom<serde_json::Value>
{
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {}, Created at: {}, conetent: [{}]",
            self.id, self.create_time, self.evt
        )
    }
}

impl From<Message> for callisto::mq_storage::Model {
    fn from(val: Message) -> Self {
        use callisto::mq_storage::Model;

        let category = match val.evt {
            EventType::ApiRequest(_) => "ApiRequestEvent".into(),

            #[allow(unreachable_patterns)]
            _ => "Unknown".into(),
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
        let evt = match value.category.as_str() {
            "ApiRequestEvent" => {
                if let Some(s) = value.content {
                    let evt = serde_json::from_str(&s).unwrap();
                    EventType::ApiRequest(evt)
                } else {
                    EventType::ErrorEvent
                }
            },

            _ => EventType::ErrorEvent
        };

        Self { id, create_time, evt }
    }
}

#[cfg(test)]
mod tests {}
