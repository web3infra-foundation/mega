//! The p2p node currently involves entering commands in the command terminal
//! and parsing them into corresponding processing logic.(see input_command.rs)
//! This method is inconvenient for testing and cannot be integrated with the UI.
//! Therefore, in the node client, we've introduced additional HTTP services to
//! interpret with user operations.

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc};

use async_std::sync::RwLock;
use axum::extract::Path;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use libp2p::Swarm;
use serde::{Deserialize, Serialize};

use crate::node::ClientParas;
use crate::{network::behaviour, node::command_handler::CmdHandler};

#[derive(Clone)]
pub struct P2pNodeState {
    pub swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    pub client_paras: Arc<RwLock<ClientParas>>,
}

pub async fn server(
    _p2p_address: String,
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<RwLock<ClientParas>>,
) {
    let state = P2pNodeState {
        swarm,
        client_paras,
    };

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/mega/:command", get(mega_handler).put(mega_handler))
                .route("/nostr", get(nostr_handler).put(nostr_handler)),
        )
        // .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from_str("127.0.0.1:8000").unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn mega_handler(
    Path(command): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = CmdHandler {
        swarm: state.swarm.clone(),
        client_paras: state.client_paras.clone(),
    };
    let repo_name = query.get("repo_name").unwrap();

    match command.as_str() {
        "provide" => cmd_handler.provide(repo_name).await,
        "search" => cmd_handler.search(repo_name).await,
        // "clone" => cmd_handler.clone(mega_address).await,
        // "clone-object" => cmd_handler.clone_obj(repo_name).await,
        // "pull" => cmd_handler.pull(mega_address).await,
        // "pull-object" => cmd_handler.pull_obj(repo_name).await,
        _ => todo!(),
    }
    Ok(Json("ok"))
}

async fn nostr_handler(
    Path(command): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd_handler = CmdHandler {
        swarm: state.swarm.clone(),
        client_paras: state.client_paras.clone(),
    };
    let repo_name = query.get("repo_name").unwrap();

    match command.as_str() {
        "subscribe" => cmd_handler.subscribe(repo_name).await,
        // "event_update" => cmd_handler.event_update(repo_name).await,
        // "event_merge" => cmd_handler.event_merge(repo_name).await,
        // "event_issue" => cmd_handler.event_issue(repo_name).await,
        _ => todo!(),
    }
    Ok(Json("ok"))
}

#[derive(Serialize, Deserialize)]
pub struct Directories {}
