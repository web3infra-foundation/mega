//! The p2p node currently involves entering commands in the command terminal
//! and parsing them into corresponding processing logic.(see input_command.rs)
//! This method is inconvenient for testing and cannot be integrated with the UI.
//! Therefore, in the node client, we've introduced additional HTTP services to
//! interpret with user operations.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;

use axum::routing::put;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct P2pNodeState {
    pub sender: Sender<String>,
}

pub async fn server(sender: Sender<String>) {
    let state = P2pNodeState { sender };

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .nest("/mega/", mega_routers())
                .nest("/nostr", nostr_routers()),
        )
        // .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from_str("0.0.0.0:8001").unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub fn mega_routers() -> Router<P2pNodeState> {
    Router::new()
        .route("/provide", put(mega_provide))
        .route("/search", get(mega_search))
        .route("/clone", get(mega_clone))
        .route("/clone-object", get(mega_clone_obj))
        .route("/pull", get(mega_pull))
        .route("/pull-object", get(mega_pull_obj))
}

async fn mega_provide(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["mega", "provide", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn mega_search(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["mega", "search", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn mega_clone(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mega_address = query.get("mega_address").unwrap();
    let line = ["mega", "clone", mega_address].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn mega_clone_obj(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["mega", "clone-object", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn mega_pull(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mega_address = query.get("mega_address").unwrap();
    let line = ["mega", "pull", mega_address].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn mega_pull_obj(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["mega", "pull-object", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

pub fn nostr_routers() -> Router<P2pNodeState> {
    Router::new()
        .route("/subscribe", get(nostr_subscribe))
        .route("/event-update", put(nostr_event_update))
        .route("/event-merge", put(nostr_event_merge))
        .route("/event-issue", put(nostr_event_issue))
}

async fn nostr_subscribe(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["nostr", "subscribe", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn nostr_event_update(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["nostr", "event-update", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn nostr_event_merge(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["nostr", "event-merge", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}

async fn nostr_event_issue(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    let line = ["nostr", "event-issue", repo_name].join(" ");
    state.0.sender.send(line).await.unwrap();
    Ok(Json("ok"))
}
