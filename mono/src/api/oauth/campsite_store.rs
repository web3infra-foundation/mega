use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use http::header::COOKIE;
use reqwest::Client;
use reqwest::Url;
use tower_sessions::{
    session::{Id, Record},
    session_store::Result,
    SessionStore,
};

use crate::api::oauth::model::CampsiteUserJson;
use crate::api::oauth::model::LoginUser;
use crate::api::oauth::CAMPSITE_API_COOKIE;

#[derive(Debug, Clone)]
pub struct CampsiteApiStore {
    client: Arc<Client>,
    // cookie_store: Arc<Jar>,
    api_base_url: String,
}

#[async_trait]
impl SessionStore for CampsiteApiStore {
    async fn save(&self, _record: &Record) -> Result<()> {
        // CampsiteApiStore is a read-only store, so we don't implement save
        Ok(())
    }

    async fn load(&self, _session_id: &Id) -> Result<Option<Record>> {
        // CampsiteApiStore doesn't store sessions by ID, it loads them from an external API
        // We'll return None here and handle the loading in a different way
        Ok(None)
    }

    async fn delete(&self, _session_id: &Id) -> Result<()> {
        // CampsiteApiStore is a read-only store, so we don't implement delete
        Ok(())
    }
}

impl CampsiteApiStore {
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

    // Custom method to load user from external API
    pub async fn load_user_from_api(
        &self,
        cookie_value: String,
    ) -> anyhow::Result<Option<LoginUser>> {
        let url = format!("{}/v1/users/me", self.api_base_url)
            .parse::<Url>()
            .context("failed to parse API base URL")?;

        let resp = self
            .client
            .get(url)
            .header(COOKIE, format!("{}={}", CAMPSITE_API_COOKIE, cookie_value))
            .send()
            .await
            .context("failed to send request to campsite API")?;

        if resp.status().is_success() {
            let campsite_user = resp
                .json::<CampsiteUserJson>()
                .await
                .context("failed to parse campsite user JSON")?;
            let login_user: LoginUser = campsite_user.into();
            Ok(Some(login_user))
        } else {
            tracing::error!("load user from API failed with status: {}", resp.status());
            Ok(None)
        }
    }
}
