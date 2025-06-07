use super::{ScoState, FAIL, SUCCESS};
use crate::manager::reset::reset_core;
use crate::manager::status::status_core;
use crate::manager::store::TempStoreArea;
use crate::util::config;
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

/// Handles the git status request.
pub(super) async fn git_status_handler(
    Query(params): Query<GitStatusParams>,
    State(state): State<ScoState>,
) -> axum::Json<GitStatus> {
    let mut status = axum::Json(GitStatus::default());
    let manager_lock = state.manager.lock().await;
    let store_path = config::store_path();
    for works in manager_lock.works.iter() {
        if works.path.eq(&params.path) {
            let work_path = PathBuf::from(store_path).join(works.hash.clone());
            let modified_path = work_path.join("modifiedstore");
            let temp_store_area = match TempStoreArea::new(&modified_path) {
                Ok(res) => res,
                Err(err) => {
                    status.status = FAIL.to_string();
                    status.message = err.to_string();
                    return status;
                }
            };
            return match status_core(&work_path, &temp_store_area) {
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

/// Handles the git commit request.
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

/// Handles the git add request.
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

#[derive(serde::Deserialize, Default)]
pub(super) struct ResetReq {
    path: String,
}

/// Handles the git reset request.
pub(super) async fn git_reset_handler(
    State(state): State<ScoState>,
    axum::Json(req): axum::Json<ResetReq>,
) -> impl IntoResponse {
    let manager_lock = state.manager.lock().await;
    let store_path = config::store_path();
    for works in manager_lock.works.iter() {
        if works.path.eq(&req.path) {
            // e.g.
            // works.path.eq("third-party/mega/scorpio")
            // ! works.path.eq("third-party/mega/scorpio/")
            let work_path = PathBuf::from(store_path).join(works.hash.clone());
            return match reset_core(&work_path) {
                Ok(_) => (axum::http::StatusCode::OK).into_response(),
                Err(err) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error: {err}"),
                )
                    .into_response(),
            };
        }
    }

    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Error: Mount dir not found.",
    )
        .into_response()
}

#[derive(serde::Deserialize)]
pub(super) struct PushRequest {
    mono_path: String,
}

/// Handles the git push request.
pub(super) async fn git_push_handler(
    State(state): State<ScoState>,
    axum::Json(payload): axum::Json<PushRequest>,
) -> impl IntoResponse {
    match state
        .manager
        .lock()
        .await
        .push_commit(&payload.mono_path)
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
