use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use http::header::COOKIE;
use reqwest::{Client, Url};
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
    session_store::Result,
};

use crate::api::oauth::model::{LoginUser, TinyshipGetSessionResponse};

static TINYSHIP_API_COOKIE: &str = "__Secure-better-auth.session_token";

#[derive(Debug, Clone)]
pub struct TinyshipApiStore {
    client: Arc<Client>,
    api_base_url: String,
}

#[async_trait]
impl SessionStore for TinyshipApiStore {
    async fn save(&self, _record: &Record) -> Result<()> {
        // TinyshipApiStore is a read-only store, so we don't implement save.
        Ok(())
    }

    async fn load(&self, _session_id: &Id) -> Result<Option<Record>> {
        // TinyshipApiStore doesn't store sessions by ID; it loads user data from external API.
        Ok(None)
    }

    async fn delete(&self, _session_id: &Id) -> Result<()> {
        // TinyshipApiStore is a read-only store, so we don't implement delete.
        Ok(())
    }
}

impl TinyshipApiStore {
    pub fn session_cookie_name(&self) -> &'static str {
        TINYSHIP_API_COOKIE
    }

    pub fn new(api_base_url: String) -> Self {
        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to build client");

        Self {
            client: Arc::new(client),
            api_base_url,
        }
    }

    pub async fn load_user_from_api(
        &self,
        cookie_value: String,
    ) -> anyhow::Result<Option<LoginUser>> {
        let base = self.api_base_url.trim_end_matches('/');
        let url = format!("{base}/api/auth/get-session")
            .parse::<Url>()
            .context("failed to parse tinyship get-session URL")?;

        let resp = self
            .client
            .get(url)
            .header(COOKIE, format!("{}={}", TINYSHIP_API_COOKIE, cookie_value))
            .send()
            .await
            .context("failed to send request to tinyship API")?;

        if !resp.status().is_success() {
            tracing::warn!("tinyship get-session: non-success status {}", resp.status());
            return Ok(None);
        }

        let body = resp
            .text()
            .await
            .context("failed to read tinyship response body")?;
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let parsed: Option<TinyshipGetSessionResponse> =
            serde_json::from_str(trimmed).context("failed to parse tinyship get-session JSON")?;

        let Some(body) = parsed else {
            return Ok(None);
        };

        let Some(user) = body.user else {
            return Ok(None);
        };

        Ok(Some(user.into()))
    }
}
