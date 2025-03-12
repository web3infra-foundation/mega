use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use common::config::Config;
use jupiter::context::Context;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use super::api;

#[derive(Clone, Debug, Parser)]
pub struct RelayOptions {
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub host: String,

    #[arg(long, default_value_t = 8001)]
    pub relay_port: u16,

    #[arg(long, short)]
    pub config: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub context: Context,
    pub relay_option: RelayOptions,
}

pub async fn run_relay_server(config: Arc<Config>, option: RelayOptions) {
    let app = app(config.clone(), option.clone()).await;

    let server_url = format!("{}:{}", option.host, option.relay_port);
    tracing::info!("start relay server: {server_url}");
    tokio::spawn(async move { gemini::p2p::relay::run(option.host, option.relay_port).await });
    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn app(config: Arc<Config>, relay_option: RelayOptions) -> Router {
    let state = AppState {
        context: Context::new(config).await,
        relay_option,
    };

    Router::new()
        .nest("/api/v1", routers().with_state(state))
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
}

pub fn routers() -> Router<AppState> {
    let router = Router::new().route("/hello", get(hello));
    // .route("/certificate", get(certificate))
    // .route("/ping", get(ping))
    // .route("/node_list", get(node_list))
    // .route("/repo_provide", post(repo_provide))
    // .route("/repo_list", get(repo_list))
    // .route("/test/send", get(send_message))
    // .route("/lfs_share", post(lfs_share))
    // .route("/lfs_list", get(lfs_list))
    // .route("/lfs_chunk", get(lfs_chunk));

    Router::new()
        .merge(router)
        .merge(api::nostr_router::routers())
        .merge(api::ca_router::routers())
}

async fn hello() -> Result<impl IntoResponse, (StatusCode, String)> {
    Ok(Json("hello relay"))
}

// pub async fn ping(
//     Query(query): Query<RelayGetParams>,
//     state: State<AppState>,
// ) -> Result<Json<RelayResultRes>, (StatusCode, String)> {
//     let storage = state.context.services.ztm_storage.clone();
//     let node: ztm_node::Model = match query.try_into() {
//         Ok(n) => n,
//         Err(_) => {
//             return Err((StatusCode::BAD_REQUEST, "invalid paras".to_string()));
//         }
//     };
//     match storage.insert_or_update_node(node).await {
//         Ok(_) => Ok(Json(RelayResultRes { success: true })),
//         Err(_) => Err((
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "invalid paras".to_string(),
//         )),
//     }
// }

// pub async fn node_list(
//     Query(_query): Query<RelayGetParams>,
//     state: State<AppState>,
// ) -> Result<Json<Vec<Node>>, (StatusCode, String)> {
//     let storage = state.context.services.ztm_storage.clone();
//     let nodelist: Vec<Node> = storage
//         .get_all_node()
//         .await
//         .unwrap()
//         .into_iter()
//         .map(|x| x.into())
//         .collect();
//     Ok(Json(nodelist))
// }

// pub async fn repo_provide(
//     state: State<AppState>,
//     Json(repo_info): Json<RepoInfo>,
// ) -> Result<Json<RelayResultRes>, (StatusCode, String)> {
//     if repo_info.identifier.is_empty() {
//         return Err((StatusCode::BAD_REQUEST, "paras invalid".to_string()));
//     }
//     let repo_info_model: ztm_repo_info::Model = repo_info.into();
//     let storage = state.context.services.ztm_storage.clone();
//     match storage.insert_or_update_repo_info(repo_info_model).await {
//         Ok(_) => Ok(Json(RelayResultRes { success: true })),
//         Err(_) => Err((
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "invalid paras".to_string(),
//         )),
//     }
// }

// pub async fn repo_list(
//     Query(_query): Query<RelayGetParams>,
//     state: State<AppState>,
// ) -> Result<Json<Vec<RepoInfo>>, (StatusCode, String)> {
//     let storage = state.context.services.ztm_storage.clone();
//     let repo_info_list: Vec<RepoInfo> = storage
//         .get_all_repo_info()
//         .await
//         .unwrap()
//         .into_iter()
//         .map(|x| x.into())
//         .collect();
//     let nodelist: Vec<Node> = storage
//         .get_all_node()
//         .await
//         .unwrap()
//         .into_iter()
//         .map(|x| x.into())
//         .collect();
//     let mut repo_info_list_result = vec![];
//     for mut repo in repo_info_list {
//         for node in &nodelist {
//             if repo.origin == node.peer_id {
//                 repo.peer_online = node.online;
//             }
//         }
//         repo_info_list_result.push(repo.clone());
//     }
//     Ok(Json(repo_info_list_result))
// }

// pub async fn lfs_share(
//     state: State<AppState>,
//     Json(lfs_info): Json<LFSInfoPostBody>,
// ) -> Result<Json<RelayResultRes>, (StatusCode, String)> {
//     let ztm_lfs_model: ztm_lfs_info::Model = lfs_info.into();
//     let storage = state.context.services.ztm_storage.clone();
//     match storage.insert_lfs_info(ztm_lfs_model).await {
//         Ok(_) => Ok(Json(RelayResultRes { success: true })),
//         Err(_) => Err((
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "invalid paras".to_string(),
//         )),
//     }
// }

// pub async fn lfs_list(
//     Query(_query): Query<RelayGetParams>,
//     state: State<AppState>,
// ) -> Result<Json<Vec<LFSInfo>>, (StatusCode, String)> {
//     let lfs_info_list_result = lfs_list_handler(state).await;
//     Ok(Json(lfs_info_list_result))
// }

// async fn lfs_list_handler(state: State<AppState>) -> Vec<LFSInfo> {
//     let storage = state.context.services.ztm_storage.clone();
//     let lfs_info_list: Vec<LFSInfo> = storage
//         .get_all_lfs_info()
//         .await
//         .unwrap()
//         .into_iter()
//         .map(|x| x.into())
//         .collect();
//     let nodelist: Vec<Node> = storage
//         .get_all_node()
//         .await
//         .unwrap()
//         .into_iter()
//         .map(|x| x.into())
//         .collect();
//     let mut lfs_info_list_result = vec![];
//     for mut lfs in lfs_info_list {
//         for node in &nodelist {
//             if lfs.peer_id == node.peer_id {
//                 lfs.peer_online = node.online;
//             }
//         }
//         lfs_info_list_result.push(lfs.clone());
//     }
//     lfs_info_list_result
// }

// async fn send_message(
//     Query(query): Query<HashMap<String, String>>,
//     state: State<AppState>,
// ) -> Result<Json<String>, (StatusCode, String)> {
//     let ztm_agent_port = state.relay_option.ztm_agent_port;
//     let peer_id = match query.get("peer_id") {
//         Some(i) => i.to_string(),
//         None => {
//             return Err((
//                 StatusCode::BAD_REQUEST,
//                 String::from("peer_id not provide\n"),
//             ));
//         }
//     };
//     let path = match query.get("path") {
//         Some(i) => i.to_string(),
//         None => {
//             return Err((StatusCode::BAD_REQUEST, String::from("path not provide\n")));
//         }
//     };
//     let result = match send_get_request_to_peer_by_tunnel(ztm_agent_port, peer_id, path).await {
//         Ok(s) => s,
//         Err(e) => {
//             tracing::error!(e);
//             return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
//         }
//     };

//     Ok(Json(result))
// }

// pub async fn lfs_chunk(
//     Query(query): Query<RelayGetParams>,
//     state: State<AppState>,
// ) -> Result<Json<LFSInfoRes>, (StatusCode, String)> {
//     if query.file_hash.is_none() {
//         return Err((StatusCode::BAD_REQUEST, "not enough paras".to_string()));
//     }
//     let file_hash = query.file_hash.unwrap().clone();

//     let lfs_object = state
//         .context
//         .services
//         .lfs_db_storage
//         .get_lfs_object(file_hash.clone())
//         .await
//         .unwrap();

//     if lfs_object.is_none() {
//         return Err((
//             StatusCode::INTERNAL_SERVER_ERROR,
//             "lfs chunk info not found".to_string(),
//         ));
//     }

//     let mut lfs_object_chunks: Vec<LFSChunk> = state
//         .context
//         .services
//         .lfs_db_storage
//         .get_lfs_relations(file_hash)
//         .await
//         .unwrap()
//         .iter()
//         .map(|x| x.clone().into())
//         .collect();

//     let mut lfs_info_res: LFSInfoRes = lfs_object.unwrap().into();
//     lfs_info_res.chunks.append(&mut lfs_object_chunks);

//     Ok(Json(lfs_info_res))
// }

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
