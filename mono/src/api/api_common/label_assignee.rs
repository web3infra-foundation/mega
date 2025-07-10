use std::collections::HashSet;

use axum::{extract::State, Json};

use common::model::CommonResult;
use jupiter::storage::stg_common::model::LabelAssigneeParams;

use crate::api::error::ApiError;
use crate::api::MonoApiServiceState;
use crate::api::{
    api_common::model::{AssigneeUpdatePayload, LabelUpdatePayload},
    oauth::model::LoginUser,
};

pub async fn label_update(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    payload: LabelUpdatePayload,
    item_type: String,
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

    let params = LabelAssigneeParams {
        item_id,
        link,
        username: user.username,
        item_type,
    };

    issue_storage
        .modify_labels(to_add, to_remove, params)
        .await?;
    Ok(Json(CommonResult::success(None)))
}

pub async fn assignees_update(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    payload: AssigneeUpdatePayload,
    item_type: String,
) -> Result<Json<CommonResult<()>>, ApiError> {
    let issue_storage = state.issue_stg();

    let AssigneeUpdatePayload {
        assignees,
        link,
        item_id,
    } = payload;

    let old_models = issue_storage
        .find_item_exist_assignees(payload.item_id)
        .await
        .unwrap();

    let old_ids: HashSet<String> = old_models.iter().map(|m| m.assignnee_id.clone()).collect();
    let new_ids: HashSet<String> = assignees.iter().cloned().collect();

    let to_add: Vec<String> = new_ids.difference(&old_ids).cloned().collect();
    let to_remove: Vec<String> = old_ids.difference(&new_ids).cloned().collect();

    let params = LabelAssigneeParams {
        item_id,
        link,
        username: user.username,
        item_type,
    };

    issue_storage
        .modify_assignees(to_add, to_remove, params)
        .await?;
    Ok(Json(CommonResult::success(None)))
}
