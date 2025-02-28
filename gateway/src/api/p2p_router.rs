use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use common::model::CommonResult;
use gemini::http::handler::{get_alias_from_identifier, get_peer_id_from_identifier};

use crate::api::MegaApiServiceState;

pub fn routers() -> Router<MegaApiServiceState> {
    Router::new()
        .route("/p2p/repo_fork", get(repo_fork))
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
    let alias = match get_alias_from_identifier(identifier.clone()) {
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
    let res = gemini::p2p::client::request_file(bootstrap_node, alias, remote_peer_id).await;
    let res = match res {
        Ok(_) => CommonResult::success(Some("ok".to_string())),
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
