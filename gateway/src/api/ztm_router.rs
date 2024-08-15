use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use common::model::CommonResult;

use crate::api::MegaApiServiceState;

pub fn routers() -> Router<MegaApiServiceState> {
    Router::new()
        .route("/ztm/repo_provide", get(repo_provide))
        .route("/ztm/repo_fork", get(repo_fork))
}

async fn repo_provide(
    Query(query): Query<HashMap<String, String>>,
    state: State<MegaApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let path = match query.get("path") {
        Some(p) => p,
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("Path not provide\n")));
        }
    };
    let bootstrap_node = match state.ztm.bootstrap_node.clone() {
        Some(b) => b.clone(),
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Bootstrap node not provide\n"),
            ));
        }
    };
    let res = match gemini::http::handler::repo_provide(
        state.port,
        bootstrap_node,
        state.inner.context.clone(),
        path.to_string(),
    )
    .await
    {
        Ok(s) => CommonResult::success(Some(s)),
        Err(err) => CommonResult::failed(err.as_str()),
    };
    Ok(Json(res))
}

async fn repo_fork(
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

    let res = gemini::http::handler::repo_fork(
        state.ztm.ztm_agent_port,
        identifier.to_string(),
        local_port,
    )
    .await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
