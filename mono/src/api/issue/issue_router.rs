use axum::{
    extract::{Path, State},
    Json,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use callisto::sea_orm_active_enums::ConvTypeEnum;
use common::model::{CommonPage, CommonResult, PageParams};
use jupiter::service::IssueService;

use crate::api::{
    api_common::model::ListPayload, conversation::SaveCommentRequest, label::LabelUpdatePayload,
};
use crate::api::{
    api_common::{self, model::AssigneeUpdatePayload},
    MonoApiServiceState,
};
use crate::api::{
    issue::{IssueDetailRes, ItemRes, NewIssue},
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
            .routes(routes!(labels))
            .routes(routes!(assignees)),
    )
}

/// Fetch Issue list
#[utoipa::path(
    post,
    path = "/list",
    request_body = PageParams<ListPayload>,
    responses(
        (status = 200, body = CommonResult<CommonPage<ItemRes>>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn fetch_issue_list(
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<ListPayload>>,
) -> Result<Json<CommonResult<CommonPage<ItemRes>>>, ApiError> {
    let (items, total) = state
        .issue_stg()
        .get_issue_list(json.additional.into(), json.pagination)
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
) -> Result<Json<CommonResult<IssueDetailRes>>, ApiError> {
    let issue_service: IssueService = state.storage.issue_service.clone();
    let issue_details = issue_service.get_issue_details(&link).await?;
    Ok(Json(CommonResult::success(Some(issue_details.into()))))
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
    let res = state
        .issue_stg()
        .save_issue(&user.username, &json.title)
        .await
        .unwrap();
    let _ = state
        .conv_stg()
        .add_conversation(
            &res.link,
            &user.username,
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
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.issue_stg().close_issue(&link).await?;
    state
        .conv_stg()
        .add_conversation(
            &link,
            &user.username,
            Some(format!("{} closed this", user.username)),
            ConvTypeEnum::Closed,
        )
        .await?;
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
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.issue_stg().reopen_issue(&link).await?;
    state
        .conv_stg()
        .add_conversation(
            &link,
            &user.username,
            Some(format!("{} reopen this", user.username)),
            ConvTypeEnum::Closed,
        )
        .await?;
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
        .conv_stg()
        .add_conversation(
            &link,
            &user.username,
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
    api_common::label_assignee::label_update(user, state, payload, String::from("issue")).await
}

/// update issue related assignees
#[utoipa::path(
    post,
    path = "/assignees",
    request_body = AssigneeUpdatePayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn assignees(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(payload): Json<AssigneeUpdatePayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    api_common::label_assignee::assignees_update(user, state, payload, String::from("issue")).await
}
