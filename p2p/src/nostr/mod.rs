use std::fmt;
use serde::{Deserialize, Serialize};

pub mod event;
pub mod kind;
pub mod tag;
pub mod client_message;
pub mod relay_message;


// ["EVENT", <event>]
// ["REQ", <subscription_id>, <filters>...]
// ["CLOSE", <subscription_id>]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NostrReq(pub String);

//["EVENT", <subscription_id>, <event>]
// ["OK", <event_id>, <true|false>, <message>]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NostrRes(pub String);

/// Messages error
#[derive(Debug)]
pub enum MessageHandleError {
    /// Invalid message format
    InvalidMessageFormat,
    /// Impossible to deserialize message
    Json(serde_json::Error),
    /// Empty message
    EmptyMsg,
    /// Event error
    Event(event::Error),
}

impl std::error::Error for MessageHandleError {}

impl fmt::Display for MessageHandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMessageFormat => write!(f, "Message has an invalid format"),
            Self::Json(e) => write!(f, "Json deserialization failed: {e}"),
            Self::EmptyMsg => write!(f, "Received empty message"),
            Self::Event(e) => write!(f, "Event: {e}"),
        }
    }
}

impl From<serde_json::Error> for MessageHandleError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<event::Error> for MessageHandleError {
    fn from(e: event::Error) -> Self {
        Self::Event(e)
    }
}

#[cfg(test)]
mod tests {}
