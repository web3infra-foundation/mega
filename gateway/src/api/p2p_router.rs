use std::collections::HashMap;

use crate::api::MegaApiServiceState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use callisto::git_repo::Model;
use common::model::CommonResult;
use gemini::util::{
    get_path_from_identifier, get_peer_id_from_identifier, repo_path_to_identifier,
};

pub fn routers() -> Router<MegaApiServiceState> {
    Router::new()
        .route("/p2p/repo_fork", get(repo_fork))
        .route("/p2p/repo_share", get(repo_share))
        .route("/p2p/peer_id", get(peer_id))
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
    let remote_peer_id = match get_peer_id_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(_e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                String::from("Identifier invalid\n"),
            ))
        }
    };
    let path = match get_path_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(_e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                String::from("Identifier invalid\n"),
            ))
        }
    };
    let bootstrap_node = match state.p2p.bootstrap_node.clone() {
        Some(b) => b,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("bootstrap node not provide\n"),
            ))
        }
    };
    let res = gemini::p2p::client::request_git_clone(
        state.inner.context.clone(),
        bootstrap_node,
        path,
        remote_peer_id,
    )
    .await;
    let res = match res {
        Ok(_) => CommonResult::success(Some("ok".to_string())),
        Err(err) => CommonResult::failed(&err.to_string()),
    };

    Ok(Json(res))
}

async fn repo_share(
    Query(query): Query<HashMap<String, String>>,
    state: State<MegaApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let path = match query.get("path") {
        Some(i) => i,
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("path not provide\n")));
        }
    };

    let _bootstrap_node = match state.p2p.bootstrap_node.clone() {
        Some(b) => b,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("bootstrap node not provide\n"),
            ))
        }
    };
    let storage = state.inner.context.services.git_db_storage.clone();
    let repo: Model = match storage
        .find_git_repo_exact_match(path.as_str())
        .await
        .unwrap()
    {
        Some(repo) => repo,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Repo path invalid\n"),
            ))
        }
    };
    let identifier = repo_path_to_identifier(repo.repo_path).await;
    let res = gemini::p2p::client::repo_share(identifier).await;
    let res = match res {
        Ok(s) => CommonResult::success(Some(s.to_string())),
        Err(err) => CommonResult::failed(&err.to_string()),
    };

    Ok(Json(res))
}

async fn peer_id(
    Query(_query): Query<HashMap<String, String>>,
    _state: State<MegaApiServiceState>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let peer_id = vault::get_peerid().await;
    Ok(Json(CommonResult::success(Some(peer_id))))
}
