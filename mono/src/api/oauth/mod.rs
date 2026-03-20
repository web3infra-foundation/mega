use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{self, Authorization, authorization::Bearer},
};
use callisto::{bot_tokens, bots};
use http::request::Parts;
use jupiter::storage::user_storage::UserStorage;
use model::LoginUser;

use crate::api::{MonoApiServiceState, oauth::api_store::OAuthApiStore};

/// Resolves `LoginUser` from a Mono-stored personal access token (e.g. Git HTTP `Authorization: Bearer`).
/// This path is independent of Campsite/Tinyship cookie session (`OAuthApiStore`).
async fn login_user_from_mono_access_token(
    user_storage: &UserStorage,
    token: &str,
) -> anyhow::Result<Option<LoginUser>> {
    if let Some(username) = user_storage.find_user_by_token(token).await? {
        return Ok(Some(LoginUser {
            username,
            ..Default::default()
        }));
    }
    Ok(None)
}

pub mod api_store;
pub mod campsite_store;
pub mod model;
pub mod tinyship_store;

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "Login first").into_response()
    }
}

pub struct BotIdentity {
    pub bot: bots::Model,
    pub token: bot_tokens::Model,
}

impl<S> FromRequestParts<S> for BotIdentity
where
    MonoApiServiceState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthRedirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization: Bearer <token> header
        let bearer = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|e| {
                tracing::debug!("BotIdentity: missing or invalid Authorization header: {e}");
                AuthRedirect
            })?
            .0
            .0;

        let raw_token = bearer.token();
        const BOT_PREFIX: &str = "bot_";

        // Enforce bot_ prefix for bot identity routes
        if !raw_token.starts_with(BOT_PREFIX) {
            tracing::debug!("BotIdentity: bearer token does not start with expected bot_ prefix");
            return Err(AuthRedirect);
        }

        // Delegate token validation to Jupiter storage (BotsStorage)
        let state_ref = MonoApiServiceState::from_ref(state);
        let bots_storage = state_ref.storage.bots_storage();

        // BotsStorage::find_bot_by_token is tolerant to presence/absence of the prefix,
        // but we pass the original token string here for clarity.
        match bots_storage.find_bot_by_token(raw_token).await {
            Ok(Some((bot, token))) => Ok(BotIdentity { bot, token }),
            Ok(None) => {
                tracing::warn!("BotIdentity: bot token not found, revoked, or expired");
                Err(AuthRedirect)
            }
            Err(e) => {
                tracing::error!("BotIdentity: error while validating bot token: {:?}", e);
                Err(AuthRedirect)
            }
        }
    }
}

impl<S> FromRequestParts<S> for LoginUser
where
    OAuthApiStore: FromRef<S>,
    UserStorage: FromRef<S>,
    S: Send + Sync,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = AuthRedirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let store = OAuthApiStore::from_ref(state);
        let user_storage = UserStorage::from_ref(state);

        // Bearer: Mono personal access token (Git HTTP / CLI), not external session cookie.
        if let Ok(TypedHeader(Authorization(bearer))) =
            parts.extract::<TypedHeader<Authorization<Bearer>>>().await
        {
            let token = bearer.token();
            match login_user_from_mono_access_token(&user_storage, token).await {
                Ok(Some(user)) => return Ok(user),
                Ok(None) => {
                    tracing::debug!("LoginUser: invalid or expired bearer token");
                    return Err(AuthRedirect);
                }
                Err(e) => {
                    tracing::warn!("LoginUser: error validating bearer token: {e:?}");
                    return Err(AuthRedirect);
                }
            }
        }

        // Cookie: external auth session (Campsite or Tinyship per `OAuthApiStore`).
        let cookies = parts
            .extract::<TypedHeader<headers::Cookie>>()
            .await
            .map_err(|e| {
                tracing::debug!("LoginUser: failed to read Cookie header: {e}");
                AuthRedirect
            })?;

        let session_cookie = cookies
            .get(store.session_cookie_name())
            .ok_or(AuthRedirect)?;

        // Load user from external API
        match store.load_user_from_api(session_cookie.to_string()).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => {
                tracing::debug!("LoginUser: invalid or expired session (external auth)");
                Err(AuthRedirect)
            }
            Err(e) => {
                tracing::warn!("LoginUser: error loading user from cookie session: {e:?}");
                Err(AuthRedirect)
            }
        }
    }
}
