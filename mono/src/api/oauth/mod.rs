use std::sync::Arc;

use anyhow::Context;
use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts, Query, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::{
    TypedHeader,
    headers::{self, Authorization, authorization::Bearer},
    typed_header::TypedHeaderRejectionReason,
};
use chrono::{Duration, Utc};
use common::config::OauthConfig;
use http::request::Parts;
use model::{GitHubUserJson, LoginUser, OauthCallbackParams};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use tower_sessions::{MemoryStore, Session, SessionStore, session::Id};
use utoipa_axum::router::OpenApiRouter;

use super::GithubClient;
use crate::api::{MonoApiServiceState, error::ApiError, oauth::campsite_store::CampsiteApiStore};

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
    let config = state.storage.config().oauth.as_ref().unwrap().clone();

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

    // Directly convert GitHub user info into our LoginUser (no local user persistence)
    let login_user: LoginUser = github_user.into();

    // Create a new session
    let session = Session::new(None, Arc::new(store.clone()), None);
    session
        .insert("user", &login_user)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to insert user into session: {:?}", e))?;

    // Save session
    session
        .save()
        .await
        .map_err(|e| anyhow::anyhow!("failed to store session: {:?}", e))?;

    // Get session cookie value
    let cookie = session
        .id()
        .ok_or_else(|| anyhow::anyhow!("Session ID not found"))?
        .to_string();

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
    let full_config = state.storage.config();
    let config = full_config.oauth.as_ref().unwrap();
    let cookie = cookies
        .get(COOKIE_NAME)
        .context("unexpected error getting cookie name")?;
    let mut headers = HeaderMap::new();

    // Parse session ID from cookie
    let session_id = cookie.parse::<Id>().map_err(|e| {
        tracing::error!("Failed to parse session ID: {:?}", e);
        anyhow::anyhow!("Invalid session ID")
    })?;

    // Delete session
    store.delete(&session_id).await.map_err(|e| {
        tracing::error!("Failed to destroy session: {:?}", e);
        anyhow::anyhow!("Failed to destroy session")
    })?;

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

    let redirect_url = format!("{ui_domain}/auth/authorized");

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

        // Try Bearer Token for CLI authentication
        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            let token = bearer.token().to_string();
            match store.load_user_from_token(token).await {
                Ok(Some(user)) => return Ok(user),
                Ok(None) => {
                    tracing::error!("Invalid or expired bearer token");
                    return Err(AuthRedirect);
                }
                Err(e) => {
                    tracing::error!("Error validating bearer token: {:?}", e);
                    return Err(AuthRedirect);
                }
            }
        }

        // Fall back to Cookie Session (UI)
        let cookies = parts
            .extract::<TypedHeader<headers::Cookie>>()
            .await
            .map_err(|e| match *e.name() {
                http::header::COOKIE => match e.reason() {
                    TypedHeaderRejectionReason::Missing => AuthRedirect,
                    _ => panic!("unexpected error getting Cookie header(s): {e}"),
                },
                _ => panic!("unexpected error getting cookies: {e}"),
            })?;

        let session_cookie = cookies.get(CAMPSITE_API_COOKIE).ok_or(AuthRedirect)?;

        // Load user from external API
        match store.load_user_from_api(session_cookie.to_string()).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => {
                tracing::error!("Invalid or expired session cookie");
                Err(AuthRedirect)
            }
            Err(e) => {
                tracing::error!("Error loading user from cookie session: {:?}", e);
                Err(AuthRedirect)
            }
        }
    }
}
