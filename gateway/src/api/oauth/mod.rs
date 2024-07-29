use std::collections::HashMap;
use std::sync::Arc;

use axum::async_trait;
use axum::response::Redirect;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use tokio::sync::Mutex;
use uuid::Uuid;

use common::enums::SupportOauthType;
use common::errors::MegaError;
use github::GithubOauthService;
use jupiter::context::Context;
use model::{AuthorizeParams, GitHubUserJson, OauthCallbackParams};

pub mod github;
pub mod model;

#[derive(Clone)]
pub struct OauthServiceState {
    pub context: Context,
    pub sessions: Arc<Mutex<HashMap<String, String>>>,
}

impl OauthServiceState {
    pub fn oauth_handler(&self, ouath_type: SupportOauthType) -> impl OauthHandler {
        match ouath_type {
            SupportOauthType::GitHub => GithubOauthService {
                context: self.context.clone(),
                client_id: self.context.config.oauth.github_client_id.clone(),
                client_secret: self.context.config.oauth.github_client_secret.clone(),
            },
        }
    }
}

#[async_trait]
pub trait OauthHandler: Send + Sync {
    fn authorize_url(&self, params: &AuthorizeParams, state: &str) -> String;

    async fn access_token(
        &self,
        params: OauthCallbackParams,
        redirect_uri: &str,
    ) -> Result<String, MegaError>;

    async fn user_info(&self, access_token: &str) -> Result<GitHubUserJson, MegaError>;
}

pub fn routers() -> Router<OauthServiceState> {
    Router::new()
        .route("/:oauth_type/authorize", get(redirect_authorize))
        .route("/:oauth_type/callback", get(oauth_callback))
        .route("/:oauth_type/user", get(user))
}

async fn redirect_authorize(
    Path(oauth_type): Path<String>,
    Query(query): Query<AuthorizeParams>,
    service_state: State<OauthServiceState>,
) -> Result<Redirect, (StatusCode, String)> {
    let oauth_type: SupportOauthType = match oauth_type.parse::<SupportOauthType>() {
        Ok(value) => value,
        Err(err) => return Err((StatusCode::BAD_REQUEST, err)),
    };

    let mut sessions = service_state.sessions.lock().await;
    let state = Uuid::new_v4().to_string();
    sessions.insert(state.clone(), query.redirect_uri.clone());
    let auth_url = service_state
        .oauth_handler(oauth_type)
        .authorize_url(&query, &state);
    Ok(Redirect::temporary(&auth_url))
}

async fn oauth_callback(
    Path(oauth_type): Path<String>,
    Query(query): Query<OauthCallbackParams>,
    service_state: State<OauthServiceState>,
) -> Result<Redirect, (StatusCode, String)> {
    let oauth_type: SupportOauthType = match oauth_type.parse::<SupportOauthType>() {
        Ok(value) => value,
        Err(err) => return Err((StatusCode::BAD_REQUEST, err)),
    };
    // chcek state,
    // TODO storage can be replaced by redis, otherwise invalid state can't be expired
    let mut sessions = service_state.sessions.lock().await;

    let redirect_uri = match sessions.get(&query.state) {
        Some(uri) => uri.clone(),
        None => return Err((StatusCode::BAD_REQUEST, "Invalid state".to_string())),
    };
    let access_token = service_state
        .oauth_handler(oauth_type)
        .access_token(query.clone(), &redirect_uri)
        .await
        .unwrap();
    sessions.remove(&query.state);

    let callback_url = format!("{}?access_token={}", redirect_uri, access_token);
    Ok(Redirect::temporary(&callback_url))
}

async fn user(
    Path(oauth_type): Path<String>,
    TypedHeader(Authorization::<Bearer>(token)): TypedHeader<Authorization<Bearer>>,
    service_state: State<OauthServiceState>,
) -> Result<Json<GitHubUserJson>, (StatusCode, String)> {
    let oauth_type: SupportOauthType = match oauth_type.parse::<SupportOauthType>() {
        Ok(value) => value,
        Err(err) => return Err((StatusCode::BAD_REQUEST, err)),
    };
    let res = service_state
        .oauth_handler(oauth_type)
        .user_info(token.token())
        .await
        .unwrap();
    Ok(Json(res))
}
