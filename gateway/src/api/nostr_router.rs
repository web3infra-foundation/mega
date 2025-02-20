use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use common::model::CommonResult;
use gemini::{
    nostr::{event::NostrEvent, relay_message::RelayMessage, GitEvent},
    util::repo_path_to_identifier,
};
use serde::{Deserialize, Serialize};

use crate::api::MegaApiServiceState;

pub fn routers() -> Router<MegaApiServiceState> {
    Router::new()
        .route("/nostr", post(recieve))
        .route("/nostr/quic/send_event", post(send_quic))
        .route("/nostr/send_event", post(send))
        .route("/nostr/event_list", get(event_list))
}

async fn recieve(
    state: State<MegaApiServiceState>,
    body: String,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    // ["EVENT", <subscription_id>, <event JSON as defined above>], used to send events requested by clients.
    // ["OK", <event_id>, <true|false>, <message>], used to indicate acceptance or denial of an EVENT message.
    // ["EOSE", <subscription_id>], used to indicate the end of stored events and the beginning of events newly received in real-time.
    // ["CLOSED", <subscription_id>, <message>], used to indicate that a subscription was ended on the server side.
    // ["NOTICE", <message>], used to send human-readable error messages or other things to clients.
    tracing::info!("nostr recieve:{}", body);
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
                .ztm_storage
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

async fn send(
    state: State<MegaApiServiceState>,
    body: String,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    tracing::info!("git event recieve:{}", body);
    let git_event_req: GitEventReq = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    };

    let git_db_storage = state.inner.context.services.git_db_storage.clone();
    let git_model = git_db_storage
        .find_git_repo_exact_match(&git_event_req.path)
        .await
        .unwrap();

    let git_model = match git_model.clone() {
        Some(r) => r,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Repo not found".to_string(),
            ))
        }
    };
    let http_port = state.port;
    let identifier = repo_path_to_identifier(http_port, git_model.clone().repo_path).await;

    let git_ref = git_db_storage
        .get_default_ref(git_model.id)
        .await
        .unwrap()
        .unwrap();

    let bootstrap_node = match state.p2p.bootstrap_node.clone() {
        Some(b) => b,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "bootstrap node is not set".to_string(),
            ));
        }
    };

    let git_event = git_event_req.to_git_event(identifier, git_ref.ref_git_id).await;

    match git_event.sent_to_relay(bootstrap_node.clone()).await {
        Ok(_) => {
            tracing::info!(
                "send event to relay({}) successfully\n{:?}",
                bootstrap_node,
                git_event
            );
        }
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }
    Ok(Json(CommonResult::success(None)))
}

async fn send_quic(
    state: State<MegaApiServiceState>,
    body: String,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let bootstrap_node = match state.p2p.bootstrap_node.clone() {
        Some(b) => b,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "bootstrap node is not set".to_string(),
            ));
        }
    };

    match gemini::p2p::client::send(
        "1".to_string(),
        "nostr_event".to_string(),
        "hello".as_bytes().to_vec(),
        bootstrap_node,
    )
    .await
    {
        Ok(_) => {}
        Err(_) => {}
    }

    Ok(Json(CommonResult::success(None)))
}

pub async fn event_list(
    Query(_query): Query<HashMap<String, String>>,
    state: State<MegaApiServiceState>,
) -> Result<Json<Vec<NostrEvent>>, (StatusCode, String)> {
    let storage = state.inner.context.services.ztm_storage.clone();
    let event_list: Vec<NostrEvent> = storage
        .get_all_nostr_event()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.try_into().unwrap())
        .collect();
    Ok(Json(event_list))
}
