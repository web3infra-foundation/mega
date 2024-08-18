use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use callisto::{ztm_node, ztm_repo_info};
use clap::Parser;
use common::config::Config;
use gemini::ztm::hub::{LocalHub, ZTMUserPermit, ZTMCA};
use gemini::ztm::send_get_request_to_peer_by_tunnel;
use gemini::{Node, RelayGetParams, RelayResultRes, RepoInfo};
use jupiter::context::Context;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use super::api;

#[derive(Clone, Debug, Parser)]
pub struct RelayOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,

    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub hub_host: String,

    #[arg(long, default_value_t = 8001)]
    pub relay_port: u16,

    #[arg(long, default_value_t = 7777)]
    pub ztm_agent_port: u16,

    #[arg(long, default_value_t = 8888)]
    pub ztm_hub_port: u16,

    #[arg(long, default_value_t = 9999)]
    pub ca_port: u16,

    #[arg(long, default_value_t = false)]
    pub only_agent: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub context: Context,
    pub relay_option: RelayOptions,
}

pub async fn run_relay_server(config: Config, option: RelayOptions) {
    let app = app(config.clone(), option.clone()).await;

    let server_url = format!("{}:{}", option.host, option.relay_port);
    tracing::info!("start relay server: {server_url}");
    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(config: Config, relay_option: RelayOptions) -> Router {
    let state = AppState {
        context: Context::new(config.clone()).await,
        relay_option,
    };

    let context = Context::new(config.clone()).await;
    tokio::spawn(async move { loop_running(context).await });
    Router::new()
        .nest("/api/v1", routers().with_state(state))
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
}

pub fn routers() -> Router<AppState> {
    let router = Router::new()
        .route("/hello", get(hello))
        .route("/certificate", get(certificate))
        .route("/ping", get(ping))
        .route("/node_list", get(node_list))
        .route("/repo_provide", post(repo_provide))
        .route("/repo_list", get(repo_list))
        .route("/test/send", get(send_message));

    Router::new()
        .merge(router)
        .merge(api::nostr_router::routers())
}

async fn hello() -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json("hello relay"))
}

async fn certificate(
    Query(query): Query<RelayGetParams>,
    state: State<AppState>,
) -> Result<Json<ZTMUserPermit>, (StatusCode, String)> {
    let option = state.relay_option.clone();
    if query.name.is_none() {
        return Err((StatusCode::BAD_REQUEST, "not enough paras".to_string()));
    }
    let name = query.name.unwrap();

    let ztm: LocalHub = LocalHub {
        hub_host: option.hub_host,
        hub_port: option.ztm_hub_port,
        ca_port: option.ca_port,
    };
    let permit = match ztm.create_ztm_certificate(name.clone()).await {
        Ok(p) => p,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
    };
    Ok(Json(permit))
}

pub async fn ping(
    Query(query): Query<RelayGetParams>,
    state: State<AppState>,
) -> Result<Json<RelayResultRes>, (StatusCode, String)> {
    let storage = state.context.services.ztm_storage.clone();
    let node: ztm_node::Model = match query.try_into() {
        Ok(n) => n,
        Err(_) => {
            return Err((StatusCode::BAD_REQUEST, "invalid paras".to_string()));
        }
    };
    match storage.insert_or_update_node(node).await {
        Ok(_) => Ok(Json(RelayResultRes { success: true })),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid paras".to_string(),
        )),
    }
}

pub async fn node_list(
    Query(_query): Query<RelayGetParams>,
    state: State<AppState>,
) -> Result<Json<Vec<Node>>, (StatusCode, String)> {
    let storage = state.context.services.ztm_storage.clone();
    let nodelist: Vec<Node> = storage
        .get_all_node()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.into())
        .collect();
    Ok(Json(nodelist))
}

pub async fn repo_provide(
    state: State<AppState>,
    Json(repo_info): Json<RepoInfo>,
) -> Result<Json<RelayResultRes>, (StatusCode, String)> {
    if repo_info.identifier.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "paras invalid".to_string()));
    }
    let repo_info_model: ztm_repo_info::Model = repo_info.into();
    let storage = state.context.services.ztm_storage.clone();
    match storage.insert_or_update_repo_info(repo_info_model).await {
        Ok(_) => Ok(Json(RelayResultRes { success: true })),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid paras".to_string(),
        )),
    }
}

pub async fn repo_list(
    Query(_query): Query<RelayGetParams>,
    state: State<AppState>,
) -> Result<Json<Vec<RepoInfo>>, (StatusCode, String)> {
    let storage = state.context.services.ztm_storage.clone();
    let repo_info_list: Vec<RepoInfo> = storage
        .get_all_repo_info()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.into())
        .collect();
    let nodelist: Vec<Node> = storage
        .get_all_node()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.into())
        .collect();
    let mut repo_info_list_result = vec![];
    for mut repo in repo_info_list {
        for node in &nodelist {
            if repo.origin == node.peer_id {
                repo.peer_online = node.online;
            }
        }
        repo_info_list_result.push(repo.clone());
    }
    Ok(Json(repo_info_list_result))
}

async fn send_message(
    Query(query): Query<HashMap<String, String>>,
    state: State<AppState>,
) -> Result<Json<String>, (StatusCode, String)> {
    let ztm_agent_port = state.relay_option.ztm_agent_port;
    let peer_id = match query.get("peer_id") {
        Some(i) => i.to_string(),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                String::from("peer_id not provide\n"),
            ));
        }
    };
    let path = match query.get("path") {
        Some(i) => i.to_string(),
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("path not provide\n")));
        }
    };
    let result = match send_get_request_to_peer_by_tunnel(ztm_agent_port, peer_id, path).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
    };

    Ok(Json(result))
}

async fn loop_running(context: Context) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        check_nodes_online(context.clone()).await;
        // ping_self(context.clone()).await;
        interval.tick().await;
    }
}

async fn check_nodes_online(context: Context) {
    let storage = context.services.ztm_storage.clone();
    let nodelist: Vec<ztm_node::Model> =
        storage.get_all_node().await.unwrap().into_iter().collect();
    for mut node in nodelist {
        //check online
        let from_timestamp = Duration::from_millis(node.last_online_time as u64);
        let now = SystemTime::now();
        let elapsed = match now.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => dur,
            Err(_) => {
                continue;
            }
        };
        if elapsed.as_secs() > from_timestamp.as_secs() + 60 {
            node.online = false;
            storage.update_node(node.clone()).await.unwrap();
        }
    }
}

// async fn ping_self(context: Context) {
//     let storage = context.services.ztm_storage.clone();
//     let nodelist: Vec<ztm_node::Model> =
//         storage.get_all_node().await.unwrap().into_iter().collect();
//     for mut node in nodelist {
//         //check online
//         let from_timestamp = Duration::from_millis(node.last_online_time as u64);
//         let now = SystemTime::now();
//         let elapsed = match now.duration_since(SystemTime::UNIX_EPOCH) {
//             Ok(dur) => dur,
//             Err(_) => {
//                 continue;
//             }
//         };
//         if elapsed.as_secs() > from_timestamp.as_secs() + 60 {
//             node.online = false;
//             storage.update_node(node.clone()).await.unwrap();
//         }
//     }
// }

#[cfg(test)]
mod tests {}
