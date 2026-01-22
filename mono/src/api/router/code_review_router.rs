use axum::{Json, extract::{Path,State}};
use ceres::model::code_review::{
    CodeReviewResponse, CommentReplyRequest, CommentReviewResponse, InitializeCommentRequest,
    ThreadReviewResponse, ThreadStatusResponse, UpdateCommentRequest,
};
use common::model::CommonResult;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser},
    server::http_server::CODE_REVIEW_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/code_review",
        OpenApiRouter::new()
            .routes(routes!(code_review_comment_list))
            .routes(routes!(initialize_code_review_comment))
            .routes(routes!(reply_code_review_comment))
            .routes(routes!(update_code_review_comment))
            .routes(routes!(resolve_code_review_thread))
            .routes(routes!(reopen_code_review_thread))
            .routes(routes!(delete_code_review_thread))
            .routes(routes!(delete_code_review_comment)),
    )
}

/// List code review comments
#[utoipa::path(
    get,
    params(
        ("link", description = "CL link"),
    ),
    path = "/{link}/comments",
    responses(
        (status = 200, body = CommonResult<CodeReviewResponse>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn code_review_comment_list(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<CodeReviewResponse>>, ApiError> {
    let comments = state
        .storage
        .code_review_service
        .get_all_comments_by_link(&link)
        .await?;

    Ok(Json(CommonResult::success(Some(comments.into()))))
}

/// Initialize a code review comment in a new thread
#[utoipa::path(
    post,
    params(
        ("link", description = "CL link"),
    ),
    path = "{/link}/comment/init",
    responses(
        (status = 200, body = CommonResult<ThreadReviewResponse>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn initialize_code_review_comment(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(paload): Json<InitializeCommentRequest>,
) -> Result<Json<CommonResult<ThreadReviewResponse>>, ApiError> {
    let thread = state
        .storage
        .code_review_service
        .create_inline_comment(
            &link,
            &paload.file_path,
            paload.line_number,
            paload.diff_side.into(),
            user.username,
            paload.content,
        )
        .await?;

    Ok(Json(CommonResult::success(Some(thread.into()))))
}

/// Reply to a code review comment
#[utoipa::path(
    post,
    params(
        ("thread_id", description = "Code Review Comment Thread ID"),
    ),
    path = "/{thread_id}/comment/reply",
    responses(
        (status = 200, body = CommonResult<CommentReviewResponse>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn reply_code_review_comment(
    user: LoginUser,
    Path(thread_id): Path<i64>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<CommentReplyRequest>,
) -> Result<Json<CommonResult<CommentReviewResponse>>, ApiError> {
    let comment = state
        .storage
        .code_review_service
        .reply_to_comment(
            thread_id,
            payload.parent_comment_id,
            user.username,
            payload.content,
        )
        .await?;

    Ok(Json(CommonResult::success(Some(comment.into()))))
}

/// Update a code review comment
#[utoipa::path(
    post,
    params(
        ("comment_id", description = "A numeric ID representing a comment"),
    ),
    path = "/{comment_id}/update",
    responses(
        (status = 200, body = CommonResult<CommentReviewResponse>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn update_code_review_comment(
    Path(comment_id): Path<i64>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<UpdateCommentRequest>,
) -> Result<Json<CommonResult<CommentReviewResponse>>, ApiError> {
    let comment = state
        .storage
        .code_review_service
        .update_comment(comment_id, payload.content)
        .await?;

    Ok(Json(CommonResult::success(Some(comment.into()))))
}

/// Resolve a code review thread
#[utoipa::path(
    post,
    params(
        ("thread_id", description = "A numeric ID representing a code review thread"),
    ),
    path = "/{thread_id}/resolve",
    responses(
        (status = 200, body = CommonResult<ThreadStatusResponse>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn resolve_code_review_thread(
    Path(thread_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<ThreadStatusResponse>>, ApiError> {
    let thread = state
        .storage
        .code_review_service
        .resolve_thread(thread_id)
        .await?;

    Ok(Json(CommonResult::success(Some(thread.into()))))
}

/// Reopen a code review thread
#[utoipa::path(
    post,
    params( 
        ("thread_id", description = "A numeric ID representing a code review thread"),
    ),
    path = "/{thread_id}/reopen",
    responses(
        (status = 200, body = CommonResult<ThreadStatusResponse>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn reopen_code_review_thread(
    Path(thread_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<ThreadStatusResponse>>, ApiError> {
    let thread = state
        .storage
        .code_review_service
        .reopen_thread(thread_id)
        .await?;

    Ok(Json(CommonResult::success(Some(thread.into()))))
}

/// Delete a code review thread and its comments
#[utoipa::path(
    delete,
    params(
        ("thread_id", description = "A numeric ID representing a code review thread"),
    ),
    path = "/thread/{thread_id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn delete_code_review_thread(
    Path(thread_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .storage
        .code_review_service
        .delete_thread(thread_id)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

/// Delete a code review comment
#[utoipa::path(
    delete,
    params(
        ("comment_id", description = "A numeric ID representing a code review comment"),
    ),
    path = "/comment/{comment_id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CODE_REVIEW_TAG,
)]
async fn delete_code_review_comment(
    Path(comment_id): Path<i64>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .storage
        .code_review_service
        .delete_comment(comment_id)
        .await?;

    Ok(Json(CommonResult::success(None)))
}