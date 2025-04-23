use crate::event::{EventBase, EventType};
use crate::queue::get_mq;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubWebhookEvent {
    pub _type: WebhookType,
    pub payload: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebhookType {
    PullRequest,
    Issues,
    Unknown(String),
}

impl From<&str> for WebhookType {
    fn from(value: &str) -> Self {
        match value {
            "pull_request" => WebhookType::PullRequest,
            "issues" => WebhookType::Issues,
            _ => WebhookType::Unknown(value.to_string()),
        }
    }
}

impl std::fmt::Display for GithubWebhookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitHub Webhook Event: {:?}", self._type)
    }
}

#[async_trait]
impl EventBase for GithubWebhookEvent {
    async fn process(&self) {
        tracing::info!("Processing: [{}]", &self);
        tracing::info!("Payload: {:#?}", &self.payload);
    }
}

impl GithubWebhookEvent {
    // Create and enqueue this event.
    pub fn notify(_type: WebhookType, payload: Value) {
        get_mq().send(EventType::GithubWebhook(GithubWebhookEvent {
            _type,
            payload,
        }));
    }
}

// For storing the data into database.
impl From<GithubWebhookEvent> for Value {
    fn from(value: GithubWebhookEvent) -> Self {
        serde_json::to_value(value).unwrap()
    }
}

impl TryFrom<Value> for GithubWebhookEvent {
    type Error = crate::event::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let res: GithubWebhookEvent = serde_json::from_value(value)?;
        Ok(res)
    }
}
