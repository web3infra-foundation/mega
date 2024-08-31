use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use common::model::CommonResult;

use crate::api::oauth::model::LoginUser;
use crate::api::user::model::AddSSHKey;
use crate::api::user::model::ListSSHKey;
use crate::api::MonoApiServiceState;

pub fn routers() -> Router<MonoApiServiceState> {
    Router::new()
        .route("/user", get(user))
        .route("/user/ssh", post(add_key))
        .route("/user/ssh/:key_id/delete", post(remove_key))
        .route("/user/ssh/list", get(list_key))
}

async fn user(
    user: LoginUser,
    _: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<LoginUser>>, (StatusCode, String)> {
    Ok(Json(CommonResult::success(Some(user))))
}

async fn add_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(json): Json<AddSSHKey>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let res = state
        .context
        .services
        .user_storage
        .save_ssh_key(user.user_id, &json.ssh_key)
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn remove_key(
    _: LoginUser,
    state: State<MonoApiServiceState>,
    Path(key_id): Path<i64>,
) -> Result<Json<CommonResult<String>>, (StatusCode, String)> {
    let res = state
        .context
        .services
        .user_storage
        .delete_ssh_key(key_id)
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn list_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<ListSSHKey>>>, (StatusCode, String)> {
    let res = state
        .context
        .services
        .user_storage
        .list_user_ssh(user.user_id)
        .await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data.into_iter().map(|x| x.into()).collect())),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}
