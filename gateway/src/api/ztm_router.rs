use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use callisto::ztm_path_mapping;
use common::model::CommonResult;
use gemini::nostr::subscribe_git_event;
use vault::get_peerid;

use crate::api::model::RepoProvideQuery;
use crate::api::MegaApiServiceState;

pub fn routers() -> Router<MegaApiServiceState> {
    Router::new()
        .route("/ztm/repo_provide", post(repo_provide))
        .route("/ztm/repo_fork", get(repo_folk))
        .route("/ztm/peer_id", get(peer_id))
}

async fn repo_provide(
    state: State<MegaApiServiceState>,
    Json(json): Json<RepoProvideQuery>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let bootstrap_node = match state.ztm.bootstrap_node.clone() {
        Some(b) => b.clone(),
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Bootstrap node not provide\n"),
            ));
        }
    };
    let RepoProvideQuery { path, alias } = json.clone();
    let context = state.inner.context.clone();
    let model: ztm_path_mapping::Model = json.into();
    match context.services.ztm_storage.save_alias_mapping(model.clone()).await {
        Ok(_) => (),
        Err(err) => return Err((StatusCode::BAD_REQUEST, err.to_string())),
    }
    let res = match gemini::http::handler::repo_provide(
        bootstrap_node,
        state.inner.context.clone(),
        path,
        alias,
    )
    .await
    {
        Ok(s) => CommonResult::success(Some(s)),
        Err(err) => CommonResult::failed(err.as_str()),
    };
    Ok(Json(res))
}

async fn repo_folk(
    Query(query): Query<HashMap<String, String>>,
    state: State<MegaApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let identifier = match query.get("identifier") {
        Some(i) => i,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                String::from("Identifier not provide\n"),
            ));
        }
    };
    let local_port = match query.get("port") {
        Some(i) => i,
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("Port not provide\n")));
        }
    };
    let local_port = match local_port.parse::<u16>() {
        Ok(i) => i,
        Err(_) => {
            return Err((StatusCode::BAD_REQUEST, String::from("Port not valid\n")));
        }
    };

    let res = gemini::http::handler::repo_folk(
        state.ztm.ztm_agent_port,
        identifier.clone().to_string(),
        local_port,
    )
    .await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };

    //nostr subscribe to Events
    if let Some(bootstrap_node) = state.ztm.bootstrap_node.clone() {
        let _ = subscribe_git_event(identifier.to_string(), get_peerid(), bootstrap_node).await;
    }

    Ok(Json(res))
}

async fn peer_id(
    Query(_query): Query<HashMap<String, String>>,
    _state: State<MegaApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let (peer_id, _) = vault::init();
    Ok(Json(CommonResult::success(Some(peer_id))))
}
