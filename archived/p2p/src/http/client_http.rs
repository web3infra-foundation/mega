//! The p2p node currently involves entering commands in the command terminal
//! and parsing them into corresponding processing logic.(see input_command.rs)
//! This method is inconvenient for testing and cannot be integrated with the UI.
//! Therefore, in the node client, we've introduced additional HTTP services to
//! interpret with user operations.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use axum::routing::put;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use storage::driver::database::storage::ObjectStorage;

use crate::network::Client;

use super::command_handler::HttpHandler;

#[derive(Clone)]
pub struct P2pNodeState {
    network_client: Client,
    storage: Arc<dyn ObjectStorage>,
    local_peer_id: String,
    relay_peer_id: String,
}

pub fn get_http_handler(state: State<P2pNodeState>) -> HttpHandler {
    HttpHandler {
        network_client: state.network_client.clone(),
        storage: state.storage.clone(),
        local_peer_id: state.local_peer_id.clone(),
        relay_peer_id: state.relay_peer_id.clone(),
    }
}

pub async fn server(
    network_client: Client,
    storage: Arc<dyn ObjectStorage>,
    local_peer_id: String,
    relay_peer_id: String,
    p2p_http_port: u16,
) {
    let state = P2pNodeState {
        network_client,
        storage,
        local_peer_id,
        relay_peer_id,
    };

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .nest("/mega", mega_routers())
                .nest("/nostr", nostr_routers())
                .route("/status", get(life_cycle_check)),
        )
        // .layer(TraceLayer::new_for_http())
        .with_state(state);

    let p2p_http_address = format!("0.0.0.0:{}", p2p_http_port);
    let addr = SocketAddr::from_str(p2p_http_address.as_str()).unwrap();
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

async fn life_cycle_check() -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json("p2p node http ready"))
}

async fn mega_provide(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    get_http_handler(state)
        .mega_provide(repo_name.clone())
        .await
}

async fn mega_search(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    get_http_handler(state).mega_search(repo_name.clone()).await
}

async fn mega_clone(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mega_address = query.get("mega_address").unwrap();
    get_http_handler(state)
        .mega_clone(mega_address.clone())
        .await
}

async fn mega_clone_obj(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    get_http_handler(state)
        .mega_clone_or_pull_obj(repo_name.clone())
        .await
}

async fn mega_pull(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mega_address = query.get("mega_address").unwrap();
    get_http_handler(state)
        .mega_pull(mega_address.clone())
        .await
}

async fn mega_pull_obj(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    get_http_handler(state)
        .mega_clone_or_pull_obj(repo_name.clone())
        .await
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
    get_http_handler(state)
        .nostr_subscribe(repo_name.clone())
        .await
}

async fn nostr_event_update(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    get_http_handler(state)
        .nostr_event_update(repo_name.clone())
        .await
}

async fn nostr_event_merge(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    get_http_handler(state)
        .nostr_event_merge(repo_name.clone())
        .await
}

async fn nostr_event_issue(
    Query(query): Query<HashMap<String, String>>,
    state: State<P2pNodeState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let repo_name = query.get("repo_name").unwrap();
    get_http_handler(state)
        .nostr_event_issue(repo_name.clone())
        .await
}

#[cfg(test)]
mod test {
    // use std::collections::HashMap;

    // use async_std::stream::StreamExt;
    // use axum::{extract::Query, http::Uri};
    // use futures::channel::mpsc;

    // use crate::node::client_http::{
    //     mega_clone, mega_clone_obj, mega_pull, mega_pull_obj, P2pNodeState,
    // };
    // use crate::node::client_http::{mega_provide, mega_search};

    // #[tokio::test]
    // async fn test_mega_routers() {
    //     let query: Query<HashMap<String, String>> = Query::try_from_uri(
    //         &"http://localhost:8001/api/v1/mega/provide?repo_name=reponame.git"
    //             .parse::<Uri>()
    //             .unwrap(),
    //     )
    //     .unwrap();

    //     let addr_query: Query<HashMap<String, String>> = Query::try_from_uri(
    //         &"http://localhost:8001/api/v1/mega/clone?mega_address=p2p://peer_id/reponame.git"
    //             .parse::<Uri>()
    //             .unwrap(),
    //     )
    //     .unwrap();

    //     let (tx, mut rx) = mpsc::channel::<String>(64);
    //     let s = P2pNodeState { sender: tx };
    //     let state = axum::extract::State(s);
    //     let _ = mega_provide(query.clone(), state.clone()).await;
    //     let _ = mega_search(query.clone(), state.clone()).await;
    //     let _ = mega_clone(addr_query.clone(), state.clone()).await;
    //     let _ = mega_clone_obj(query.clone(), state.clone()).await;
    //     let _ = mega_pull(addr_query.clone(), state.clone()).await;
    //     let _ = mega_pull_obj(query.clone(), state.clone()).await;

    //     assert_eq!(rx.next().await.unwrap(), "mega provide reponame.git");
    //     assert_eq!(rx.next().await.unwrap(), "mega search reponame.git");
    //     assert_eq!(
    //         rx.next().await.unwrap(),
    //         "mega clone p2p://peer_id/reponame.git"
    //     );
    //     assert_eq!(rx.next().await.unwrap(), "mega clone-object reponame.git");
    //     assert_eq!(
    //         rx.next().await.unwrap(),
    //         "mega pull p2p://peer_id/reponame.git"
    //     );
    //     assert_eq!(rx.next().await.unwrap(), "mega pull-object reponame.git");
    // }
}
