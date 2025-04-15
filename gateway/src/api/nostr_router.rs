use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use common::model::CommonResult;
use gemini::nostr::{event::NostrEvent, relay_message::RelayMessage, GitEvent};

use crate::api::MegaApiServiceState;

pub fn routers() -> Router<MegaApiServiceState> {
    Router::new()
        .route("/nostr", post(receive))
        .route("/nostr/event_list", get(event_list))
}

async fn receive(
    state: State<MegaApiServiceState>,
    body: String,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    // ["EVENT", <subscription_id>, <event JSON as defined above>], used to send events requested by clients.
    // ["OK", <event_id>, <true|false>, <message>], used to indicate acceptance or denial of an EVENT message.
    // ["EOSE", <subscription_id>], used to indicate the end of stored events and the beginning of events newly received in real-time.
    // ["CLOSED", <subscription_id>, <message>], used to indicate that a subscription was ended on the server side.
    // ["NOTICE", <message>], used to send human-readable error messages or other things to clients.
    tracing::info!("nostr receive:{}", body);
    let relay_msg: RelayMessage = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    };
    if let RelayMessage::Event {
        subscription_id,
        event,
    } = relay_msg
    {
        if subscription_id.to_string() != vault::init().await.0 {
            return Err((StatusCode::BAD_REQUEST, String::from("bad subscription id")));
        }
        //save event to database
        if let Ok(ztm_nostr_event) = event.try_into() {
            let _ = state
                .inner
                .context
                .services
                .relay_storage
                .insert_nostr_event(ztm_nostr_event)
                .await;
        };
    }

    Ok(Json(CommonResult::success(None)))
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GitEventReq {
    // TODO change path with alias
    pub path: String,
    pub action: String,
    pub title: String,
    pub content: String,
}

impl GitEventReq {
    pub async fn to_git_event(&self, identifier: String, commit: String) -> GitEvent {
        GitEvent {
            peer: vault::get_peerid().await,
            uri: identifier,
            action: self.action.clone(),
            r#ref: "".to_string(),
            commit,
            issue: "".to_string(),
            mr: "".to_string(),
            title: self.title.clone(),
            content: self.content.clone(),
        }
    }
}

pub async fn event_list(
    Query(_query): Query<HashMap<String, String>>,
    state: State<MegaApiServiceState>,
) -> Result<Json<Vec<NostrEvent>>, (StatusCode, String)> {
    let storage = state.inner.context.services.relay_storage.clone();
    let event_list: Vec<NostrEvent> = storage
        .get_all_nostr_event()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.try_into().unwrap())
        .collect();
    Ok(Json(event_list))
}
