//! The p2p node currently involves entering commands in the command terminal
//! and parsing them into corresponding processing logic.(see input_command.rs)
//! This method is inconvenient for testing and cannot be integrated with the UI.
//! Therefore, in the node client, we've introduced additional HTTP services to
//! interpret with user operations.

use std::net::SocketAddr;
use std::str::FromStr;
use std::{collections::HashMap, sync::Arc};

use axum::routing::put;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use libp2p::Swarm;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::node::ClientParas;
use crate::{network::behaviour, node::command_handler::CmdHandler};

#[derive(Clone)]
pub struct P2pNodeState {
    pub swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    pub client_paras: Arc<Mutex<ClientParas>>,
}

pub async fn server(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
) {
    let state = P2pNodeState {
        swarm,
        client_paras,
    };

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

pub fn get_cmd_handler(state: State<P2pNodeState>) -> CmdHandler {
    CmdHandler {
        swarm: state.swarm.clone(),
        client_paras: state.client_paras.clone(),
    }
}

async fn mega_provide(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.provide(repo_name).await;
    Ok(Json("ok"))
}

async fn mega_search(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.search(repo_name).await;
    Ok(Json("ok"))
}

async fn mega_clone(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let mega_address = query.get("mega_address").unwrap();
    cmd_handler.clone(mega_address).await;
    Ok(Json("ok"))
}

async fn mega_clone_obj(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.clone_obj(repo_name).await;
    Ok(Json("ok"))
}

async fn mega_pull(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let mega_address = query.get("mega_address").unwrap();
    cmd_handler.pull(mega_address).await;
    Ok(Json("ok"))
}

async fn mega_pull_obj(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.clone(repo_name).await;
    Ok(Json("ok"))
}

pub fn nostr_routers() -> Router<P2pNodeState> {
    Router::new()
        .route("/subscribe", get(nostr_subscribe))
        .route("/event_update", put(nostr_event_update))
        .route("/event_merge", put(nostr_event_merge))
        .route("/event_issue", put(nostr_event_issue))
}

async fn nostr_subscribe(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.subscribe(repo_name).await;
    Ok(Json("ok"))
}

async fn nostr_event_update(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.event_update(repo_name).await;
    Ok(Json("ok"))
}

async fn nostr_event_merge(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.event_merge(repo_name).await;
    Ok(Json("ok"))
}

async fn nostr_event_issue(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = get_cmd_handler(state);
    let repo_name = query.get("repo_name").unwrap();
    cmd_handler.event_issue(repo_name).await;
    Ok(Json("ok"))
}

#[derive(Serialize, Deserialize)]
pub struct Directories {}
