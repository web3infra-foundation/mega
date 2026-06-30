use api_model::common::CommonResult;
use axum::{Json, extract::State};
use ceres::model::{change_list::AssigneeUpdatePayload, label::LabelUpdatePayload};

use crate::api::{MonoApiServiceState, error::ApiError, oauth::model::LoginUser};

pub async fn label_update(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    payload: LabelUpdatePayload,
    item_type: String,
) -> Result<Json<CommonResult<()>>, ApiError> {
    let LabelUpdatePayload {
        label_ids,
        link,
        item_id,
    } = payload;

    state
        .monorepo()
        .update_item_labels(&user.username, item_id, &item_type, label_ids, &link)
        .await?;

    Ok(Json(CommonResult::success(None)))
}

pub async fn assignees_update(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    payload: AssigneeUpdatePayload,
    item_type: String,
) -> Result<Json<CommonResult<()>>, ApiError> {
    let AssigneeUpdatePayload {
        assignees,
        link,
        item_id,
    } = payload;

    state
        .monorepo()
        .update_item_assignees(&user.username, item_id, &item_type, assignees, &link)
        .await?;

    Ok(Json(CommonResult::success(None)))
}
