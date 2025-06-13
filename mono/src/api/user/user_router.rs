use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json,
};
use russh::keys::{parse_public_key_base64, HashAlg};
use utoipa_axum::router::OpenApiRouter;

use common::model::CommonResult;

use crate::api::user::model::AddSSHKey;
use crate::api::user::model::ListSSHKey;
use crate::api::user::model::ListToken;
use crate::api::MonoApiServiceState;
use crate::api::{error::ApiError, oauth::model::LoginUser, util};

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new().nest(
        "/user",
        OpenApiRouter::new()
            .route("/", get(user))
            .route("/ssh", get(list_key))
            .route("/ssh", post(add_key))
            .route("/ssh/{key_id}/delete", post(remove_key))
            .route("/token/generate", post(generate_token))
            .route("/token/list", get(list_token))
            .route("/token/{key_id}/delete", post(remove_token))
            .route("/repo-permissions", get(repo_permissions)),
    )
}

async fn user(
    user: LoginUser,
    _: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<LoginUser>>, ApiError> {
    Ok(Json(CommonResult::success(Some(user))))
}

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

    let res = state
        .user_stg()
        .save_ssh_key(
            user.campsite_user_id,
            &title,
            &json.ssh_key,
            &key.fingerprint(HashAlg::Sha256).to_string(),
        )
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn remove_key(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(key_id): Path<i64>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state
        .user_stg()
        .delete_ssh_key(user.campsite_user_id, key_id)
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
) -> Result<Json<CommonResult<Vec<ListSSHKey>>>, ApiError> {
    let res = state.user_stg().list_user_ssh(user.campsite_user_id).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data.into_iter().map(|x| x.into()).collect())),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn generate_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state.user_stg().generate_token(user.campsite_user_id).await;
    let res = match res {
        Ok(data) => CommonResult::success(Some(data)),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn remove_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
    Path(key_id): Path<i64>,
) -> Result<Json<CommonResult<String>>, ApiError> {
    let res = state
        .user_stg()
        .delete_token(user.campsite_user_id, key_id)
        .await;
    let res = match res {
        Ok(_) => CommonResult::success(None),
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
}

async fn list_token(
    user: LoginUser,
    state: State<MonoApiServiceState>,
) -> Result<Json<CommonResult<Vec<ListToken>>>, ApiError> {
    let res = state.user_stg().list_token(user.campsite_user_id).await;
    let res = match res {
        Ok(data) => {
            let res = data.into_iter().map(|x| x.into()).collect();
            CommonResult::success(Some(res))
        }
        Err(err) => CommonResult::failed(&err.to_string()),
    };
    Ok(Json(res))
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
