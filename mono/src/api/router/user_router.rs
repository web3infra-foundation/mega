use axum::{
    Json,
    extract::{Path, State},
    routing::get,
};
use russh::keys::{HashAlg, parse_public_key_base64};
use utoipa_axum::{router::OpenApiRouter, routes};

use ceres::model::user::{AddSSHKey, ListSSHKey, ListToken};
use common::{errors::MegaError, model::CommonResult};

use crate::api::MonoApiServiceState;
use crate::api::{error::ApiError, oauth::model::LoginUser};
use crate::server::http_server::USER_TAG;

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/user",
        OpenApiRouter::new()
            .route("/", get(user))
            .routes(routes!(list_key))
            .routes(routes!(add_key))
            .routes(routes!(remove_key))
            .routes(routes!(generate_token))
            .routes(routes!(list_token))
            .routes(routes!(remove_token)),
    )
}

async fn user(
    user: LoginUser,
    _: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<LoginUser>>, ApiError> {
    Ok(Json(CommonResult::success(Some(user))))
}

/// Add SSH Key
#[utoipa::path(
    post,
    path = "/ssh",
    request_body = AddSSHKey,
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn add_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Json(json): Json<AddSSHKey>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let ssh_parts: Vec<&str> = json.ssh_key.split_whitespace().collect();
    let key = parse_public_key_base64(
        ssh_parts
            .get(1)
            .ok_or_else(|| MegaError::Other("Invalid key format".to_string()))?,
    )?;
    let title = if json.title.is_empty() {
        ssh_parts
            .get(2)
            .ok_or_else(|| MegaError::Other("Invalid key format".to_string()))?
            .to_string()
    } else {
        json.title
    };
    state
        .user_stg()
        .save_ssh_key(
            user.username,
            &title,
            &json.ssh_key,
            &key.fingerprint(HashAlg::Sha256).to_string(),
        )
        .await?;
    Ok(Json(CommonResult::success(None)))
}

/// Delete SSH Key
#[utoipa::path(
    delete,
        params(
        ("key_id", description = "A numeric ID representing a SSH"),
    ),
    path = "/ssh/{key_id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn remove_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(key_id): Path<i64>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state
        .user_stg()
        .delete_ssh_key(user.username, key_id)
        .await?;
    Ok(Json(CommonResult::success(None)))
}

/// Get User's SSH key list
#[utoipa::path(
    get,
    path = "/ssh/list",
    responses(
        (status = 200, body = CommonResult<Vec<ListSSHKey>>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn list_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<ListSSHKey>>>, ApiError> {
    let res = state.user_stg().list_user_ssh(user.username).await?;
    Ok(Json(CommonResult::success(Some(
        res.into_iter().map(|x| x.into()).collect(),
    ))))
}

/// Generate Token For http push
#[utoipa::path(
    post,
    path = "/token/generate",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn generate_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.user_stg().generate_token(user.username).await?;
    Ok(Json(CommonResult::success(Some(res))))
}

/// Delete User's http push token
#[utoipa::path(
    delete,
        params(
        ("key_id", description = "A numeric ID representing a User Token"),
    ),
    path = "/token/{key_id}",
    responses(
        (status = 200, body = CommonResult<String>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn remove_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(key_id): Path<i64>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    state.user_stg().delete_token(user.username, key_id).await?;
    Ok(Json(CommonResult::success(None)))
}

/// Get User's push token list
#[utoipa::path(
    get,
    path = "/token/list",
    responses(
        (status = 200, body = CommonResult<Vec<ListToken>>, content_type = "application/json")
    ),
    tag = USER_TAG
)]
async fn list_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<ListToken>>>, ApiError> {
    let data = state.user_stg().list_token(user.username).await?;
    let res = data.into_iter().map(|x| x.into()).collect();
    Ok(Json(CommonResult::success(Some(res))))
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    #[test]
    fn test_parse_all_cedar_file() {
        let path = PathBuf::from("/project/mega/src");
        for component in path.ancestors() {
            if component != Path::new("/") {
                println!("{:?}", component.join(".mega_cedar.json"));
            }
        }
    }
}
