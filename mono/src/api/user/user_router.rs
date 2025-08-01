use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json,
};
use russh::keys::{parse_public_key_base64, HashAlg};
use utoipa_axum::{router::OpenApiRouter, routes};

use common::model::CommonResult;

use crate::api::user::model::ListSSHKey;
use crate::api::user::model::ListToken;
use crate::api::MonoApiServiceState;
use crate::api::{error::ApiError, oauth::model::LoginUser, util};
use crate::{api::user::model::AddSSHKey, server::https_server::USER_TAG};

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
            .routes(routes!(remove_token))
            .route("/repo-permissions", get(repo_permissions)),
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
    let ssh_key: Vec<&str> = json.ssh_key.split_whitespace().collect();
    let key = parse_public_key_base64(ssh_key.get(1).ok_or("Invalid key format").unwrap())?;
    let title = if !json.title.is_empty() {
        json.title
    } else {
        ssh_key
            .get(2)
            .ok_or("Invalid key format")
            .unwrap()
            .to_owned()
            .to_owned()
    };

    state
        .user_stg()
        .save_ssh_key(
            user.campsite_user_id,
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
        .delete_ssh_key(user.campsite_user_id, key_id)
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
    let res = state
        .user_stg()
        .list_user_ssh(user.campsite_user_id)
        .await?;
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
    let res = state
        .user_stg()
        .generate_token(user.campsite_user_id)
        .await?;
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
    state
        .user_stg()
        .delete_token(user.campsite_user_id, key_id)
        .await?;
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
    let data = state.user_stg().list_token(user.campsite_user_id).await?;
    let res = data.into_iter().map(|x| x.into()).collect();
    Ok(Json(CommonResult::success(Some(res))))
}

async fn repo_permissions(
    Query(query): Query<HashMap<String, String>>,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let path = std::path::PathBuf::from(query.get("path").unwrap());
    let _ = util::get_entitystore(path, state).await;
    // TODO
    Ok(Json(CommonResult::success(Some(String::new()))))
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
