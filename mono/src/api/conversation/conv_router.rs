use axum::{
    extract::{Path, State},
    Json,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use common::model::CommonResult;

use crate::api::conversation::ReactionRequest;
use crate::api::oauth::model::LoginUser;
use crate::api::MonoApiServiceState;
use crate::{api::error::ApiError, server::https_server::CONV_TAG};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/conversation",
        OpenApiRouter::new().routes(routes!(comment_reactions)),
    )
}

/// Add comment reactions with emoji
#[utoipa::path(
    post,
    params(
        ("comment_id", description = "comment conversation id"),
    ),
    path = "/comments/{comment_id}/reactions",
    request_body = ReactionRequest,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = CONV_TAG
)]
async fn comment_reactions(
    user: LoginUser,
    Path(comment_id): Path<i64>,
    state: State<MonoApiServiceState>,
    Json(payload): Json<ReactionRequest>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .conv_stg()
        .add_reactions(
            Some(payload.content),
            comment_id,
            &payload.comment_type,
            &user.username,
        )
        .await?;
    Ok(Json(CommonResult::success(None)))
}
