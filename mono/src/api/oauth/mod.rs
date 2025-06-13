use anyhow::Context;
use async_session::{MemoryStore, Session, SessionStore};
use axum::{
    extract::{FromRef, FromRequestParts, Query, State},
    http::{header::SET_COOKIE, HeaderMap},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    RequestPartsExt,
};
use axum_extra::{headers, typed_header::TypedHeaderRejectionReason, TypedHeader};
use callisto::user;
use chrono::{Duration, Utc};
use http::{header, request::Parts, StatusCode};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};

use common::config::OauthConfig;
use model::{GitHubUserJson, LoginUser, OauthCallbackParams};
use utoipa_axum::router::OpenApiRouter;

use crate::api::MonoApiServiceState;
use crate::api::{error::ApiError, oauth::campsite_store::CampsiteApiStore};

use super::GithubClient;

pub mod campsite_store;
pub mod model;

static COOKIE_NAME: &str = "SESSION";

static CAMPSITE_API_COOKIE: &str = "_campsite_api_session";

pub fn routers() -> OpenApiRouter<MonoApiServiceState> {
    OpenApiRouter::new()
        .route("/github", get(github_auth))
        .route("/authorized", get(login_authorized))
        .route("/logout", get(logout))
}

async fn github_auth(State(client): State<GithubClient>) -> impl IntoResponse {
    // Issue for adding check to this example https://github.com/tokio-rs/axum/issues/2511
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    Redirect::to(auth_url.as_ref())
}

async fn login_authorized(
    Query(query): Query<OauthCallbackParams>,
    State(state): State<MonoApiServiceState>,
    State(oauth_client): State<GithubClient>,
) -> Result<impl IntoResponse, ApiError> {
    let store: MemoryStore = MemoryStore::from_ref(&state);
    let config = state.context.config.oauth.as_ref().unwrap();

    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    // Get an auth token
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(&http_client)
        .await
        .context("failed in sending request request to authorization server")?;

    // Fetch user data
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("User-Agent", format!("Mega/{}", "0.0.1"))
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .context("failed in sending request to target Url")?;
    let mut github_user = GitHubUserJson::default();

    if resp.status().is_success() {
        github_user = resp
            .json::<GitHubUserJson>()
            .await
            .context("failed to deserialize response as JSON")?;
    } else {
        tracing::error!("github:user_info:err {:?}", resp.text().await.unwrap());
    }

    let new_user: user::Model = github_user.into();
    let user = state
        .user_stg()
        .find_user_by_email(&new_user.email)
        .await
        .unwrap();

    let login_user: LoginUser;
    if let Some(user) = user {
        // Create a new session filled with user data
        login_user = user.into();
    } else {
        state.user_stg().save_user(new_user.clone()).await.unwrap();
        login_user = new_user.into();
    }

    let mut session = Session::new();
    session
        .insert("user", &login_user)
        .context("failed in inserting serialized value into session")?;

    // Store session and get corresponding cookie
    let cookie = store
        .store_session(session)
        .await
        .context("failed to store session")?
        .context("unexpected error retrieving cookie value")?;

    // SameSite=Lax: Allow GET, disable POST cookie send, prevent CSRF
    // SameSite=None: allow Post cookie send
    let cookie = format!(
        "{COOKIE_NAME}={cookie}; Domain={}; SameSite=Lax; Secure; Path=/",
        config.cookie_domain
    );
    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        cookie.parse().context("failed to parse cookie")?,
    );

    Ok((headers, Redirect::to(&config.ui_domain)))
}

async fn logout(
    State(state): State<MonoApiServiceState>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> Result<impl IntoResponse, ApiError> {
    let store: MemoryStore = MemoryStore::from_ref(&state);
    let full_config = state.context.config.clone();
    let config = full_config.oauth.as_ref().unwrap();
    let cookie = cookies
        .get(COOKIE_NAME)
        .context("unexpected error getting cookie name")?;
    let mut headers = HeaderMap::new();

    let session = match store
        .load_session(cookie.to_string())
        .await
        .context("failed to load session")?
    {
        Some(s) => s,
        // No session active, just redirect
        None => return Ok((headers, Redirect::to(config.ui_domain.as_str()))),
    };

    store
        .destroy_session(session)
        .await
        .context("failed to destroy session")?;

    // Expire cookie
    let cookie = format!(
        "{COOKIE_NAME}={cookie}; Expires={} Domain={}; SameSite=Lax; Path=/",
        config.cookie_domain,
        (Utc::now() - Duration::days(1)).to_rfc2822(),
    );
    headers.insert(
        SET_COOKIE,
        cookie.parse().context("failed to parse cookie")?,
    );
    Ok((headers, Redirect::to(&config.ui_domain)))
}

pub fn oauth_client(oauth_config: OauthConfig) -> Result<GithubClient, ApiError> {
    let client_id = oauth_config.github_client_id;
    let client_secret = oauth_config.github_client_secret;
    let ui_domain = oauth_config.ui_domain;

    let redirect_url = format!("{}/auth/authorized", ui_domain);

    let auth_url = "https://github.com/login/oauth/authorize".to_string();

    let token_url = "https://github.com/login/oauth/access_token".to_string();

    let client = GithubClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(auth_url)?)
        .set_token_uri(TokenUrl::new(token_url)?)
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(RedirectUrl::new(redirect_url)?);

    Ok(client)
}

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "Login first").into_response()
    }
}

impl<S> FromRequestParts<S> for LoginUser
where
    CampsiteApiStore: FromRef<S>,
    S: Send + Sync,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = AuthRedirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let store = CampsiteApiStore::from_ref(state);

        let cookies = parts
            .extract::<TypedHeader<headers::Cookie>>()
            .await
            .map_err(|e| match *e.name() {
                header::COOKIE => match e.reason() {
                    TypedHeaderRejectionReason::Missing => AuthRedirect,
                    _ => panic!("unexpected error getting Cookie header(s): {e}"),
                },
                _ => panic!("unexpected error getting cookies: {e}"),
            })?;
        let session_cookie = cookies.get(CAMPSITE_API_COOKIE).ok_or(AuthRedirect)?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .unwrap()
            .ok_or(AuthRedirect)?;

        let user = session.get::<LoginUser>("user").ok_or(AuthRedirect)?;

        Ok(user)
    }
}
