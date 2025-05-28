use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use common::model::{CommonPage, CommonResult, PageParams};

use crate::api::issue::{IssueDetail, IssueItem, NewIssue};
use crate::api::mr::SaveCommentRequest;
use crate::api::MonoApiServiceState;
use crate::{api::error::ApiError, server::https_server::ISSUE_TAG};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/issue",
        OpenApiRouter::new()
            .routes(routes!(fetch_issue_list))
            .routes(routes!(new_issue))
            .routes(routes!(close_issue))
            .routes(routes!(reopen_issue))
            .routes(routes!(issue_detail))
            .routes(routes!(save_comment))
            .routes(routes!(delete_comment)),
    )
}

#[derive(Deserialize, ToSchema)]
pub struct StatusParams {
    pub status: String,
}

/// Fetch Issue list
#[utoipa::path(
    post,
    path = "/list",
    request_body = PageParams<StatusParams>,
    responses(
        (status = 200, body = CommonResult<CommonPage<IssueItem>>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
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

/// Get issue details
#[utoipa::path(
    get,
    params(
        ("link", description = "Issue link"),
    ),
    path = "/{link}/detail",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
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

/// New Issue
#[utoipa::path(
    post,
    path = "/new",
    request_body = NewIssue,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn new_issue(
    // user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(json): Json<NewIssue>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let stg = state.issue_stg().clone();
    let res = stg.save_issue(0, &json.title).await.unwrap();
    let res = stg
        .add_issue_conversation(&res.link, 0, Some(json.description))
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

/// Close an issue
#[utoipa::path(
    post,
    params(
        ("link", description = "Issue link"),
    ),
    path = "/{link}/close",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn close_issue(
    // _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = match state.issue_stg().close_issue(&link).await {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

/// Reopen an issue
#[utoipa::path(
    post,
    params(
        ("link", description = "Issue link"),
    ),
    path = "/{link}/reopen",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn reopen_issue(
    // _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = match state.issue_stg().reopen_issue(&link).await {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

/// Add new comment on Issue
#[utoipa::path(
    post,
    params(
        ("link", description = "Issue link"),
    ),
    path = "/{link}/comment",
    request_body = SaveCommentRequest,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn save_comment(
    // user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<SaveCommentRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = match state
        .issue_stg()
        .add_issue_conversation(&link, 0, Some(payload.content))
        .await
    {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

/// Delete Issue Comment
#[utoipa::path(
    delete,
    params(
        ("id", description = "Conversation id"),
    ),
    path = "/comment/{id}/delete",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
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
