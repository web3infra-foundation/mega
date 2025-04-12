use super::{ScoState, FAIL, SUCCESS};
use crate::manager::status::status_core;
use crate::util::scorpio_config;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use mercury::internal::object::commit::Commit;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(serde::Serialize, Default)]
pub(super) struct GitStatus {
    status: String,
    mono_path: String,
    upper_path: String,
    lower_path: String,
    message: String,
}
#[derive(serde::Deserialize, Default)]
pub(super) struct GitStatusParams {
    path: String,
}

pub(super) async fn git_status_handler(
    Query(params): Query<GitStatusParams>,
    State(state): State<ScoState>,
) -> axum::Json<GitStatus> {
    let mut status = axum::Json(GitStatus::default());
    let manager_lock = state.manager.lock().await;
    let store_path = scorpio_config::store_path();
    for works in manager_lock.works.iter() {
        if works.path.eq(&params.path) {
            let work_path = PathBuf::from(store_path).join(works.hash.clone());
            let modified_path = work_path.join("modifiedstore");
            let index_db = sled::open(modified_path.join("index.db")).unwrap();
            let rm_db = sled::open(modified_path.join("removedfile.db")).unwrap();
            return match status_core(&work_path, &index_db, &rm_db) {
                Ok(res) => axum::Json(GitStatus {
                    status: SUCCESS.to_string(),
                    mono_path: params.path,
                    upper_path: format!("{}/upper", work_path.display()),
                    lower_path: format!("{}/lower", work_path.display()),
                    message: *res,
                }),
                Err(err) => {
                    status.status = FAIL.to_string();
                    status.message = err.to_string();
                    status
                }
            };
        }
    }

    status.status = FAIL.to_string();
    status
}

#[derive(Deserialize)]
pub(super) struct CommitPayload {
    mono_path: String,
    message: String,
}

#[derive(Serialize)]
pub(super) struct CommitResp {
    status: String,
    commit: Option<Commit>,
    msg: String,
}
#[axum::debug_handler]
pub(super) async fn git_commit_handler(
    State(state): State<ScoState>,
    axum::Json(payload): axum::Json<CommitPayload>,
) -> axum::Json<CommitResp> {
    let c = state
        .manager
        .lock()
        .await
        .mono_commit(payload.mono_path, payload.message)
        .await;
    match c {
        Ok(commit) => axum::Json(CommitResp {
            status: SUCCESS.to_owned(),
            commit: Some(commit),
            msg: SUCCESS.to_owned(),
        }),
        Err(err) => axum::Json(CommitResp {
            status: FAIL.to_owned(),
            commit: None,
            msg: err.to_string(),
        }),
    }
}

#[derive(serde::Deserialize)]
pub(super) struct AddReq {
    mono_path: String,
}

pub(super) async fn git_add_handler(
    State(state): State<ScoState>,
    axum::Json(req): axum::Json<AddReq>,
) -> impl IntoResponse {
    let path = req.mono_path;
    let res = state.manager.lock().await.mono_add(&path).await;
    match res {
        Ok(()) => (axum::http::StatusCode::OK).into_response(),
        Err(err) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {err}"),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
pub(super) struct PushRequest {
    monopath: String,
}

pub(super) async fn git_push_handler(
    State(state): State<ScoState>,
    axum::Json(payload): axum::Json<PushRequest>,
) -> impl IntoResponse {
    match state
        .manager
        .lock()
        .await
        .push_commit(&payload.monopath)
        .await
    {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::OK {
                println!("[scorpio]: push success!");
                (axum::http::StatusCode::OK, "Push successful").into_response()
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Push failed with status: {}", response.status()),
                )
                    .into_response()
            }
        }
        Err(e) => {
            eprintln!("Error during push: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error: {}", e),
            )
                .into_response()
        }
    }
}
