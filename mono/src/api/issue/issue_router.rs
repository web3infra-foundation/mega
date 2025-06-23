use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use callisto::sea_orm_active_enums::ConvTypeEnum;
use common::model::{CommonPage, CommonResult, PageParams};

use crate::api::MonoApiServiceState;
use crate::api::{issue::LabelUpdatePayload, mr::SaveCommentRequest};
use crate::api::{
    issue::{IssueDetail, IssueItem, NewIssue},
    oauth::model::LoginUser,
};
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
            .routes(routes!(delete_comment))
            .routes(routes!(labels)),
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
    let (items, total) = state
        .issue_stg()
        .get_issue_by_status(&json.additional.status, json.pagination)
        .await?;
    Ok(Json(CommonResult::success(Some(CommonPage {
        items: items.into_iter().map(|m| m.into()).collect(),
        total,
    }))))
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
    let res = state.issue_stg().get_issue(&link).await?;
    let res = if let Some(model) = res {
        let mut detail: IssueDetail = model.into();
        let conversations = state.issue_stg().get_conversations(&link).await.unwrap();
        detail.conversations = conversations.into_iter().map(|x| x.into()).collect();
        CommonResult::success(Some(detail))
    } else {
        CommonResult::success(None)
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
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(json): Json<NewIssue>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let stg = state.issue_stg().clone();
    let res = stg
        .save_issue(&user.campsite_user_id, &json.title)
        .await
        .unwrap();
    let _ = stg
        .add_conversation(
            &res.link,
            &user.campsite_user_id,
            Some(json.description),
            ConvTypeEnum::Comment,
        )
        .await?;
    Ok(Json(CommonResult::success(None)))
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
    _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.issue_stg().close_issue(&link).await?;
    Ok(Json(CommonResult::success(None)))
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
    _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.issue_stg().reopen_issue(&link).await?;
    Ok(Json(CommonResult::success(None)))
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
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<SaveCommentRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .issue_stg()
        .add_conversation(
            &link,
            &user.campsite_user_id,
            Some(payload.content),
            ConvTypeEnum::Comment,
        )
        .await?;
    Ok(Json(CommonResult::success(None)))
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
    _: LoginUser,
    Path(id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.issue_stg().remove_conversation(id).await?;
    Ok(Json(CommonResult::success(None)))
}

/// update issue related labels
#[utoipa::path(
    post,
    path = "/labels",
    request_body = LabelUpdatePayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn labels(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<LabelUpdatePayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    common_label_update(user, state, payload).await
}

pub async fn common_label_update(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    payload: LabelUpdatePayload,
) -> Result<Json<CommonResult<()>>, ApiError> {
    let issue_storage = state.issue_stg();

    let LabelUpdatePayload {
        label_ids,
        link,
        item_id,
    } = payload;

    let old_labels = issue_storage
        .find_item_exist_labels(payload.item_id)
        .await
        .unwrap();

    let old_ids: HashSet<i64> = old_labels.iter().map(|l| l.label_id).collect();
    let new_ids: HashSet<i64> = label_ids.iter().copied().collect();

    let to_add: Vec<i64> = new_ids.difference(&old_ids).copied().collect();
    let to_remove: Vec<i64> = old_ids.difference(&new_ids).copied().collect();

    issue_storage
        .modify_labels(&user.campsite_user_id, item_id, &link, to_add, to_remove)
        .await?;
    Ok(Json(CommonResult::success(None)))
}
