use axum::{
    Json,
    extract::{Path, State},
};
use ceres::model::label::{LabelItem, NewLabel};
use common::model::{CommonPage, CommonResult, PageParams};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser},
    server::http_server::LABEL_TAG,
};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/label",
        OpenApiRouter::new()
            .routes(routes!(new_label))
            .routes(routes!(fetch_label_list))
            .routes(routes!(fetch_label)),
    )
}

/// List label in page
#[utoipa::path(
    post,
    path = "/list",
    request_body = PageParams<String>,
    responses(
        (status = 200, body = CommonResult<CommonPage<LabelItem>>, content_type = "application/json")
    ),
    tag = LABEL_TAG
)]
async fn fetch_label_list(
    state: State<MonoApiServiceState>,
    Json(json): Json<PageParams<String>>,
) -> Result<Json<CommonResult<CommonPage<LabelItem>>>, ApiError> {
    let (items, total) = state
        .issue_stg()
        .list_labels_by_page(json.pagination, &json.additional)
        .await?;
    Ok(Json(CommonResult::success(Some(CommonPage {
        items: items.into_iter().map(|m| m.into()).collect(),
        total,
    }))))
}

/// New label
#[utoipa::path(
    post,
    path = "/new",
    request_body = NewLabel,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = LABEL_TAG
)]
async fn new_label(
    _user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(json): Json<NewLabel>,
) -> Result<Json<CommonResult<LabelItem>>, ApiError> {
    let stg = state.issue_stg().clone();
    let res = stg
        .new_label(&json.name, &json.color, &json.description)
        .await?;
    Ok(Json(CommonResult::success(Some(res.into()))))
}

/// Fetch label details
#[utoipa::path(
    get,
        params(
        ("id", description = "Label's id"),
    ),
    path = "/{id}",
    responses(
        (status = 200, body = CommonResult<LabelItem>, content_type = "application/json")
    ),
    tag = LABEL_TAG
)]
async fn fetch_label(
    state: State<MonoApiServiceState>,
    Path(id): Path<i64>,
) -> Result<Json<CommonResult<LabelItem>>, ApiError> {
    let label = state.issue_stg().get_label_by_id(id).await?;
    Ok(Json(CommonResult::success(label.map(|m| m.into()))))
}
