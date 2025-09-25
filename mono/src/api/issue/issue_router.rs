use axum::{
    Json,
    extract::{Path, Query, State},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use callisto::sea_orm_active_enums::ConvTypeEnum;
use common::model::{CommonPage, CommonResult, PageParams};
use jupiter::service::issue_service::IssueService;

use crate::api::{
    MonoApiServiceState,
    api_common::{self, model::AssigneeUpdatePayload},
};
use crate::api::{
    api_common::model::ListPayload,
    conversation::ContentPayload,
    issue::{IssueSuggestions, QueryPayload},
    label::LabelUpdatePayload,
};
use crate::api::{
    issue::{IssueDetailRes, ItemRes, NewIssue},
    oauth::model::LoginUser,
};
use crate::{api::error::ApiError, server::http_server::ISSUE_TAG};

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
            .routes(routes!(labels))
            .routes(routes!(assignees))
            .routes(routes!(edit_title))
            .routes(routes!(issue_suggester)),
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
        (status = 200, body = CommonResult<IssueDetailRes>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn issue_detail(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<IssueDetailRes>>, ApiError> {
    let issue_service: IssueService = state.storage.issue_service.clone();
    let issue_details: IssueDetailRes = issue_service
        .get_issue_details(&link, user.username)
        .await?
        .into();
    Ok(Json(CommonResult::success(Some(issue_details))))
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
    request_body = ContentPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn save_comment(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ContentPayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    state
        .conv_stg()
        .add_conversation(
            &link,
            &user.username,
            Some(payload.content.clone()),
            ConvTypeEnum::Comment,
        )
        .await?;
    api_common::comment::check_comment_ref(user, state, &payload.content, &link).await
}

/// Update issue related labels
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

/// Update issue related assignees
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

/// Edit issue title
#[utoipa::path(
    post,
    params(
        ("link", description = "A string ID representing a Issue"),
    ),
    path = "/{link}/title",
    request_body = ContentPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn edit_title(
    _: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ContentPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .issue_stg()
        .edit_title(&link, &payload.content)
        .await?;
    Ok(Json(CommonResult::success(None)))
}

/// Get issue suggester in comment
#[utoipa::path(
    get,
    params(QueryPayload),
    path = "/issue_suggester",
    responses(
        (status = 200, body = CommonResult<Vec<IssueSuggestions>>, content_type = "application/json")
    ),
    tag = ISSUE_TAG
)]
async fn issue_suggester(
    Query(payload): Query<QueryPayload>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<IssueSuggestions>>>, ApiError> {
    let (issues, mrs) = state
        .storage
        .issue_service
        .get_suggestions(&payload.query)
        .await?;
    let mut res: Vec<IssueSuggestions> = issues.into_iter().map(|m| m.into()).collect();
    let mut mr_list: Vec<IssueSuggestions> = mrs.into_iter().map(|m| m.into()).collect();
    res.append(&mut mr_list);
    res.sort();
    Ok(Json(CommonResult::success(Some(res))))
}
