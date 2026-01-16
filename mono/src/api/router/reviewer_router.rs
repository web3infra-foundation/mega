use axum::{
    Json,
    extract::{Path, State},
};
use callisto::sea_orm_active_enums::{ConvTypeEnum, MergeStatusEnum};
use ceres::model::change_list::{
    ChangeReviewStatePayload, ChangeReviewerStatePayload, ReviewerInfo, ReviewerPayload,
    ReviewersResponse,
};
use common::{errors::MegaError, model::CommonResult};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser},
    server::http_server::CL_TAG,
};

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
        .storage
        .reviewer_storage()
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
            .conv_stg()
            .add_conversation(
                &link,
                &user.username,
                Some(format!(
                    "{} assigned a new reviewer {}",
                    user.username, reviewer
                )),
                ConvTypeEnum::Comment,
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
        .storage
        .reviewer_storage()
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
            .conv_stg()
            .add_conversation(
                &link,
                &user.username,
                Some(format!("{} removed reviewer {}", user.username, reviewer)),
                ConvTypeEnum::Comment,
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
    let reviewers = state
        .storage
        .reviewer_storage()
        .list_reviewers(&link)
        .await?
        .into_iter()
        .map(|r| ReviewerInfo {
            username: r.username,
            approved: r.approved,
            system_required: r.system_required,
        })
        .collect();

    Ok(Json(CommonResult::success(Some(ReviewersResponse {
        result: reviewers,
    }))))
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
    let res = state.cl_stg().get_cl(&link).await?;
    let model = res.ok_or(MegaError::Other("CL Not Found".to_string()))?;

    if model.status == MergeStatusEnum::Draft {
        return Err(ApiError::from(MegaError::Other(
            ERR_CL_NOT_READY_FOR_REVIEW.to_owned(),
        )));
    }

    state
        .storage
        .reviewer_storage()
        .reviewer_change_state(&link, &user.username, payload.approved)
        .await?;

    state
        .conv_stg()
        .add_conversation(
            &link,
            &user.username,
            Some(format!("{} approved the CL", user.username)),
            ConvTypeEnum::Approve,
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
    let res = state
        .storage
        .reviewer_storage()
        .is_reviewer(&link, &user.username)
        .await?;

    if !res {
        return Err(ApiError::from(MegaError::Other(
            "Only reviewer can resolve the review comments".to_string(),
        )));
    }

    state
        .storage
        .conversation_storage()
        .change_review_state(&link, &payload.conversation_id, payload.resolved)
        .await?;

    state
        .conv_stg()
        .add_conversation(
            &link,
            &user.username,
            Some(format!("{} resolved a review", user.username)),
            ConvTypeEnum::Comment,
        )
        .await?;

    Ok(Json(CommonResult::success(None)))
}
