use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use serde::Deserialize;

use common::model::{CommonPage, CommonResult, PageParams};

use crate::api::error::ApiError;
use crate::api::issue::{IssueDetail, IssueItem, NewIssue};
use crate::api::oauth::model::LoginUser;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    Router::new().nest(
        "/issue",
        Router::new()
            .route("/list", post(fetch_issue_list))
            .route("/new", post(new_issue))
            .route("/{link}/close", post(close_issue))
            .route("/{link}/reopen", post(reopen_issue))
            .route("/{link}/detail", get(issue_detail))
            .route("/{link}/comment", post(save_comment))
            .route("/comment/{id}/delete", post(delete_comment)),
    )
}

#[derive(Deserialize)]
pub struct StatusParams {
    pub status: String,
}

async fn fetch_issue_list(
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<StatusParams>>,
) -> Result<Json<CommonResult<CommonPage<IssueItem>>>, ApiError> {
    let res = state
        .issue_stg()
        .get_issue_by_status(&json.additional.status, json.pagination)
        .await;
    let res = match res {
        Ok((items, total)) => CommonResult::success(Some(CommonPage {
            items: items.into_iter().map(|m| m.into()).collect(),
            total,
        })),

        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn issue_detail(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<IssueDetail>>, ApiError> {
    let res = match state.issue_stg().get_issue(&link).await {
        Ok(data) => {
            if let Some(model) = data {
                let mut detail: IssueDetail = model.into();
                let conversations = state.mr_stg().get_mr_conversations(&link).await.unwrap();
                detail.conversations = conversations.into_iter().map(|x| x.into()).collect();
                CommonResult::success(Some(detail))
            } else {
                CommonResult::success(None)
            }
        }
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn new_issue(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(json): Json<NewIssue>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let stg = state.issue_stg().clone();
    let res = stg.save_issue(user.user_id, &json.title).await.unwrap();
    let res = stg
        .add_issue_conversation(&res.link, user.user_id, Some(json.description))
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn close_issue(
    _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = match state.issue_stg().close_issue(&link).await {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn reopen_issue(
    _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = match state.issue_stg().reopen_issue(&link).await {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn save_comment(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    body: Bytes,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let json_string =
        String::from_utf8(body.to_vec()).unwrap_or_else(|_| "Invalid UTF-8".to_string());
    let res = match state
        .issue_stg()
        .add_issue_conversation(&link, user.user_id, Some(json_string))
        .await
    {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn delete_comment(
    Path(id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.issue_stg().remove_issue_conversation(id).await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
