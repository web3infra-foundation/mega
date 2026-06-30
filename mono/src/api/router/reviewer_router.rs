use api_model::common::CommonResult;
use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::{
    change_list::{
        ChangeReviewStatePayload, ChangeReviewerStatePayload, MergeStatus, ReviewerPayload,
        ReviewersResponse,
    },
    conversation::ConvType,
};
use common::errors::MegaError;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::api::{MonoApiServiceState, api_doc::CL_TAG, error::ApiError, oauth::model::LoginUser};

const ERR_CL_NOT_READY_FOR_REVIEW: &str = "CL is not ready for review";

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/cl",
        OpenApiRouter::new()
            .routes(routes!(add_reviewers))
            .routes(routes!(remove_reviewers))
            .routes(routes!(list_reviewers))
            .routes(routes!(reviewer_approve))
            .routes(routes!(review_resolve)),
    )
}

#[utoipa::path(
    post,
    params(
        ("link", description = "the cl link"),
    ),
    path = "/{link}/reviewers",
    request_body = ReviewerPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CL_TAG
)]
async fn add_reviewers(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ReviewerPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .monorepo()
        .add_reviewers(&link, payload.reviewer_usernames.clone())
        .await?;

    // Audit log
    tracing::info!(
        "[Audit] event=reviewer_added cl_link={} reviewers={:?} actor={}",
        link,
        payload.reviewer_usernames,
        user.username
    );

    for reviewer in payload.reviewer_usernames {
        state
            .monorepo()
            .add_conversation(
                &link,
                &user.username,
                Some(format!(
                    "{} assigned a new reviewer {}",
                    user.username, reviewer
                )),
                ConvType::Comment,
            )
            .await?;
    }

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    delete,
    params (
        ("link", description = "the cl link"),
    ),
    path = "/{link}/reviewers",
    request_body = ReviewerPayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CL_TAG
)]
async fn remove_reviewers(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ReviewerPayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .monorepo()
        .remove_reviewers(&link, &payload.reviewer_usernames)
        .await?;

    // Audit log
    tracing::info!(
        "[Audit] event=reviewer_removed cl_link={} reviewers={:?} actor={}",
        link,
        payload.reviewer_usernames,
        user.username
    );

    for reviewer in &payload.reviewer_usernames {
        state
            .monorepo()
            .add_conversation(
                &link,
                &user.username,
                Some(format!("{} removed reviewer {}", user.username, reviewer)),
                ConvType::Comment,
            )
            .await?;
    }

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    get,
    params (
        ("link", description = "the cl link")
    ),
    path = "/{link}/reviewers",
    responses(
        (status = 200, body = CommonResult<ReviewersResponse>, content_type = "application/json")
    ),
    tag = CL_TAG
)]
async fn list_reviewers(
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<ReviewersResponse>>, ApiError> {
    let reviewers = state.monorepo().list_reviewers(&link).await?;

    Ok(Json(CommonResult::success(Some(reviewers))))
}

/// Change the reviewer approval state
#[utoipa::path(
    post,
    params (
        ("link", description = "the cl link")
    ),
    path = "/{link}/reviewer/approve",
    request_body = ChangeReviewerStatePayload,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CL_TAG
)]
async fn reviewer_approve(
    user: LoginUser,
    Path(link): Path<String>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ChangeReviewerStatePayload>,
) -> Result<Json<CommonResult<()>>, ApiError> {
    if state.monorepo().cl_merge_status(&link).await? == MergeStatus::Draft {
        return Err(ApiError::from(MegaError::Other(
            ERR_CL_NOT_READY_FOR_REVIEW.to_owned(),
        )));
    }

    state
        .monorepo()
        .reviewer_change_state(&link, &user.username, payload.approved)
        .await?;

    state
        .monorepo()
        .add_conversation(
            &link,
            &user.username,
            Some(format!("{} approved the CL", user.username)),
            ConvType::Approve,
        )
        .await?;

    Ok(Json(CommonResult::success(None)))
}

#[utoipa::path(
    post,
    params (
        ("link", description = "the cl link")
    ),
    path = "/{link}/review/resolve",
    request_body (
        content = ChangeReviewStatePayload,
    ),
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CL_TAG
)]
async fn review_resolve(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(link): Path<String>,
    Json(payload): Json<ChangeReviewStatePayload>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.monorepo().is_reviewer(&link, &user.username).await?;

    if !res {
        return Err(ApiError::from(MegaError::Other(
            "Only reviewer can resolve the review comments".to_string(),
        )));
    }

    state
        .monorepo()
        .change_review_state(&link, &payload.conversation_id, payload.resolved)
        .await?;

    state
        .monorepo()
        .add_conversation(
            &link,
            &user.username,
            Some(format!("{} resolved a review", user.username)),
            ConvType::Comment,
        )
        .await?;

    Ok(Json(CommonResult::success(None)))
}
