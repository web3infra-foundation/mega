use callisto::relay_nostr_req;
use client_message::{ClientMessage, Filter, SubscriptionId};
use reqwest::{header::CONTENT_TYPE, Client};
use serde::{Deserialize, Serialize};
use std::fmt;
use tag::{Tag, TagKind};

use crate::util::handle_response;

pub mod client_message;
pub mod event;
pub mod kind;
pub mod relay_message;
pub mod tag;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GitEventReq {
    pub path: String,
    pub action: String,
    pub title: String,
    pub content: String,
}

impl GitEventReq {
    pub async fn to_git_event(
        &self,
        peer_id: String,
        identifier: String,
        commit: String,
    ) -> GitEvent {
        GitEvent {
            peer: peer_id,
            uri: identifier,
            action: self.action.clone(),
            r#ref: "".to_string(),
            commit,
            issue: "".to_string(),
            cl: "".to_string(),
            title: self.title.clone(),
            content: self.content.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Req {
    pub subscription_id: String,
    pub filters: Vec<Filter>,
}

impl From<relay_nostr_req::Model> for Req {
    fn from(n: relay_nostr_req::Model) -> Self {
        let filters: Vec<Filter> = serde_json::from_str(&n.filters).unwrap();
        Req {
            subscription_id: n.subscription_id,
            filters,
        }
    }
}

impl Req {
    pub fn filters_json(&self) -> String {
        serde_json::to_string(&self.filters).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GitEvent {
    pub peer: String,
    pub uri: String,
    pub action: String,
    pub r#ref: String,
    pub commit: String,
    pub issue: String,
    pub cl: String,
    pub title: String,
    pub content: String,
}

impl GitEvent {
    pub fn to_tags(&self) -> Vec<Tag> {
        let mut tags: Vec<Tag> = Vec::new();
        if !self.peer.is_empty() {
            let tag = Tag::Generic(TagKind::Peer, vec![self.peer.clone()]);
            tags.push(tag);
        }
        if !self.uri.is_empty() {
            let tag = Tag::Generic(TagKind::URI, vec![self.uri.clone()]);
            tags.push(tag);
        }
        if !self.action.is_empty() {
            let tag = Tag::Generic(TagKind::Action, vec![self.action.clone()]);
            tags.push(tag);
        }
        if !self.r#ref.is_empty() {
            let tag = Tag::Generic(TagKind::Ref, vec![self.r#ref.clone()]);
            tags.push(tag);
        }
        if !self.commit.is_empty() {
            let tag = Tag::Generic(TagKind::Commit, vec![self.commit.clone()]);
            tags.push(tag);
        }
        if !self.issue.is_empty() {
            let tag = Tag::Generic(TagKind::Issue, vec![self.issue.clone()]);
            tags.push(tag);
        }
        if !self.cl.is_empty() {
            let tag = Tag::Generic(TagKind::CL, vec![self.cl.clone()]);
            tags.push(tag);
        }
        if !self.title.is_empty() {
            let tag = Tag::Generic(TagKind::Title, vec![self.title.clone()]);
            tags.push(tag);
        }

        tags
    }

    pub fn from_tags(tags: Vec<Tag>) -> Self {
        let mut git_event = Self {
            peer: "".to_string(),
            uri: "".to_string(),
            action: "".to_string(),
            r#ref: "".to_string(),
            commit: "".to_string(),
            issue: "".to_string(),
            cl: "".to_string(),
            title: "".to_string(),
            content: "".to_string(),
        };
        for x in tags {
            let vec = x.as_vec();
            if vec.len() > 1 {
                let kind = TagKind::from(vec[0].clone());
                let content = vec[1].clone();
                match kind {
                    TagKind::Peer => git_event.peer = content,
                    TagKind::URI => git_event.uri = content,
                    TagKind::Action => git_event.action = content,
                    TagKind::Ref => git_event.r#ref = content,
                    TagKind::Commit => git_event.commit = content,
                    TagKind::Issue => git_event.issue = content,
                    TagKind::CL => git_event.cl = content,
                    TagKind::Title => git_event.title = content,
                    _ => {}
                }
            }
        }
        git_event
    }
}

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

pub async fn subscribe_git_event(
    uri: String,
    subscription_id: String,
    bootstrap_node: String,
) -> Result<(), String> {
    let filters = vec![Filter::new().repo_uri(uri)];

    let client_req = ClientMessage::new_req(SubscriptionId::new(subscription_id), filters);

    //send to relay
    let client = Client::new();
    let url = format!("{bootstrap_node}/api/v1/nostr");
    let request_result = client
        .post(url.clone())
        .header(CONTENT_TYPE, "application/json")
        .body(client_req.as_json())
        .send()
        .await;

    match handle_response(request_result).await {
        Ok(_s) => {
            tracing::info!("subscribe git_event successfully: {}", url);
            Ok(())
        }
        Err(e) => {
            tracing::error!("subscribe git_event successfully failed:\n{}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {}
