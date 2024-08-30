use std::env;

use anyhow::Context;
use async_session::{MemoryStore, Session, SessionStore};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Query, State},
    http::{header::SET_COOKIE, HeaderMap},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    RequestPartsExt, Router,
};
use axum_extra::{headers, typed_header::TypedHeaderRejectionReason, TypedHeader};
use http::{header, request::Parts, StatusCode};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};

use common::config::OauthConfig;
use model::{GitHubUserJson, OauthCallbackParams};

use crate::api::MonoApiServiceState;

pub mod model;

static COOKIE_NAME: &str = "SESSION";


pub fn routers() -> Router<MonoApiServiceState> {
    Router::new()
        .route("/github", get(github_auth))
        .route("/authorized", get(login_authorized))
        .route("/logout", get(logout))
}

async fn github_auth(State(client): State<BasicClient>) -> impl IntoResponse {
    // Issue for adding check to this example https://github.com/tokio-rs/axum/issues/2511
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    Redirect::to(auth_url.as_ref())
}

async fn login_authorized(
    Query(query): Query<OauthCallbackParams>,
    State(store): State<MemoryStore>,
    State(oauth_client): State<BasicClient>,
) -> Result<impl IntoResponse, OauthError> {
    // Get an auth token
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(async_http_client)
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
    let mut user_data = GitHubUserJson::default();

    if resp.status().is_success() {
        user_data = resp
            .json::<GitHubUserJson>()
            .await
            .context("failed to deserialize response as JSON")?;
    } else {
        tracing::error!("github:user_info:err {:?}", resp.text().await.unwrap());
    }
    // Create a new session filled with user data
    let mut session = Session::new();
    session
        .insert("user", &user_data)
        .context("failed in inserting serialized value into session")?;

    // Store session and get corresponding cookie
    let cookie = store
        .store_session(session)
        .await
        .context("failed to store session")?
        .context("unexpected error retrieving cookie value")?;

    // Build the cookie
    let cookie = format!("{COOKIE_NAME}={cookie}; SameSite=Lax; Path=/");

    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        cookie.parse().context("failed to parse cookie")?,
    );

    Ok((headers, Redirect::to("/")))
}

async fn logout(
    State(store): State<MemoryStore>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> Result<impl IntoResponse, OauthError> {
    let cookie = cookies
        .get(COOKIE_NAME)
        .context("unexpected error getting cookie name")?;

    let session = match store
        .load_session(cookie.to_string())
        .await
        .context("failed to load session")?
    {
        Some(s) => s,
        // No session active, just redirect
        None => return Ok(Redirect::to("/")),
    };

    store
        .destroy_session(session)
        .await
        .context("failed to destroy session")?;

    Ok(Redirect::to("/"))
}

pub fn oauth_client(oauth_config: OauthConfig) -> Result<BasicClient, OauthError> {
    let client_id = oauth_config.github_client_id;
    let client_secret = oauth_config.github_client_secret;

    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:8000/auth/authorized".to_string());

    let auth_url = env::var("AUTH_URL")
        .unwrap_or_else(|_| "https://github.com/login/oauth/authorize".to_string());

    let token_url = env::var("TOKEN_URL")
        .unwrap_or_else(|_| "https://github.com/login/oauth/access_token".to_string());

    Ok(BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url).context("failed to create new authorization server URL")?,
        Some(TokenUrl::new(token_url).context("failed to create new token endpoint URL")?),
    )
    .set_redirect_uri(
        RedirectUrl::new(redirect_url).context("failed to create new redirection URL")?,
    ))
}

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "Login in first").into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for GitHubUserJson
where
    MemoryStore: FromRef<S>,
    S: Send + Sync,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = AuthRedirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let store = MemoryStore::from_ref(state);

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
        let session_cookie = cookies.get(COOKIE_NAME).ok_or(AuthRedirect)?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .unwrap()
            .ok_or(AuthRedirect)?;

        let user = session.get::<GitHubUserJson>("user").ok_or(AuthRedirect)?;

        Ok(user)
    }
}

// Use anyhow, define error and enable '?'
// For a simplified example of using anyhow in axum check /examples/anyhow-error-response
#[derive(Debug)]
pub struct OauthError(anyhow::Error);

impl IntoResponse for OauthError {
    fn into_response(self) -> Response {
        tracing::error!("Application error: {:#}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "Login in first").into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, OauthError>`. That way you don't need to do that manually.
impl<E> From<E> for OauthError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
