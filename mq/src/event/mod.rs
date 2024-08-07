use std::{any::Any, fmt::Display};

use api_request::ApiRequestEvent;
use chrono::{DateTime, Utc};
use thiserror::Error;

pub mod api_request;

#[derive(Debug)]
pub enum EventType {
    ApiRequest(ApiRequestEvent),
}

pub struct Message {
    pub(crate) id: u64,
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

impl Display for Message{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {}, Created at: {}, conetent: [{}]",
            self.id, self.create_time, self.evt
        )
    }
}
