use std::net::SocketAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use axum::body::Body;
use axum::extract::{FromRequest, Query, State};
use axum::http::{Request, Response, StatusCode, Uri};
use axum::routing::get;
use axum::{Json, Router};
use callisto::{ztm_node, ztm_repo_info};
use common::config::Config;
use common::model::CommonOptions;
use gemini::ztm::hub::{LocalHub, ZTMCA};
use gemini::{Node, RelayGetParams, RelayResultRes, RepoInfo};
use jupiter::context::Context;
use regex::Regex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;

use crate::api::api_router::{self};
use crate::api::ApiServiceState;

pub async fn run_relay_server(config: Config, common: CommonOptions) {
    let host = common.host.clone();
    let relay_port = common.relay_port;
    let hub_port = common.ztm_hub_port;
    let ca_port = common.ca_port;
    let app = app(config.clone(), host.clone(), relay_port, hub_port, ca_port).await;

    let server_url = format!("{}:{}", host, relay_port);
    tracing::info!("start relay server: {server_url}");
    let addr = SocketAddr::from_str(&server_url).unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct AppState {
    pub context: Context,
    pub host: String,
    pub relay_port: u16,
    pub hub_port: u16,
    pub ca_port: u16,
}

pub async fn app(
    config: Config,
    host: String,
    relay_port: u16,
    hub_port: u16,
    ca_port: u16,
) -> Router {
    let state = AppState {
        host,
        relay_port,
        hub_port,
        ca_port,
        context: Context::new(config.clone()).await,
    };

    let api_state = ApiServiceState {
        context: Context::new(config).await,
    };

    let context = api_state.context.clone();
    tokio::spawn(async move { loop_running(context).await });
    // add RequestDecompressionLayer for handle gzip encode
    // add TraceLayer for log record
    // add CorsLayer to add cors header
    Router::new()
        .nest("/api/v1", api_router::routers().with_state(api_state))
        .route(
            "/*path",
            get(get_method_router).post(post_method_router),
            // .put(put_method_router),
        )
        .layer(ServiceBuilder::new().layer(CorsLayer::new().allow_origin(Any)))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(state)
}

async fn get_method_router(
    state: State<AppState>,
    Query(params): Query<RelayGetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/hello$").unwrap().is_match(uri.path()) {
        return hello_relay(params).await;
    } else if Regex::new(r"/certificate$").unwrap().is_match(uri.path()) {
        return certificate(state, params).await;
    } else if Regex::new(r"/ping$").unwrap().is_match(uri.path()) {
        return ping(state, params).await;
    } else if Regex::new(r"/node_list$").unwrap().is_match(uri.path()) {
        return node_list(state, params).await;
    } else if Regex::new(r"/repo_list$").unwrap().is_match(uri.path()) {
        return repo_list(state, params).await;
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}

async fn post_method_router(
    state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let _ztm_config = state.context.config.ztm.clone();
    if Regex::new(r"/repo_provide$").unwrap().is_match(uri.path()) {
        return repo_provide(state, req).await;
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}

pub async fn hello_relay(_params: RelayGetParams) -> Result<Response<Body>, (StatusCode, String)> {
    Ok(Response::builder().body(Body::from("hello relay")).unwrap())
}

pub async fn certificate(
    state: State<AppState>,
    params: RelayGetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    if params.name.is_none() {
        return Err((StatusCode::BAD_REQUEST, "not enough paras".to_string()));
    }
    let name = params.name.unwrap();

    let ztm: LocalHub = LocalHub {
        host: state.host.clone(),
        hub_port: state.hub_port,
        ca_port: state.ca_port,
    };
    let permit = match ztm.create_ztm_certificate(name.clone()).await {
        Ok(p) => p,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
    };

    let permit_json = serde_json::to_string(&permit).unwrap();
    tracing::info!("new permit [{name}]: {permit_json}");

    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(permit_json))
        .unwrap())
}

pub async fn ping(
    state: State<AppState>,
    params: RelayGetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    let storage = state.context.services.ztm_storage.clone();
    let node: ztm_node::Model = match params.try_into() {
        Ok(n) => n,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid paras".to_string(),
            ));
        }
    };
    match storage.insert_or_update_node(node).await {
        Ok(_) => {
            let res = serde_json::to_string(&RelayResultRes { success: true }).unwrap();
            Ok(Response::builder()
                .header("Content-Type", "application/json")
                .body(Body::from(res))
                .unwrap())
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid paras".to_string(),
        )),
    }
}

pub async fn node_list(
    state: State<AppState>,
    _params: RelayGetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    let storage = state.context.services.ztm_storage.clone();
    let nodelist: Vec<Node> = storage
        .get_all_node()
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.into())
        .collect();
    let json_string = serde_json::to_string(&nodelist).unwrap();
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(json_string))
        .unwrap())
}

pub async fn repo_provide(
    state: State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let storage = state.context.services.ztm_storage.clone();
    let request = Json::from_request(req, &state)
        .await
        .unwrap_or_else(|_| Json(RepoInfo::default()));
    let repo_info: RepoInfo = request.0;
    if repo_info.identifier.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "paras invalid".to_string()));
    }
    let repo_info_model: ztm_repo_info::Model = repo_info.into();
    match storage.insert_or_update_repo_info(repo_info_model).await {
        Ok(_) => {
            let res = serde_json::to_string(&RelayResultRes { success: true }).unwrap();
            Ok(Response::builder()
                .header("Content-Type", "application/json")
                .body(Body::from(res))
                .unwrap())
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid paras".to_string(),
        )),
    }
}

pub async fn repo_list(
    state: State<AppState>,
    _params: RelayGetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
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
    let json_string = serde_json::to_string(&repo_info_list_result).unwrap();
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(json_string))
        .unwrap())
}

async fn loop_running(context: Context) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        check_nodes_online(context.clone()).await;
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

#[cfg(test)]
mod tests {}
