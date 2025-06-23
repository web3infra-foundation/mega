use std::sync::Arc;

use anyhow::Context;
use async_session::{async_trait, Result, Session, SessionStore};
use http::header::COOKIE;
use reqwest::Client;
use reqwest::Url;

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
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        let mut session = Session::new();
        let url = format!("{}/v1/users/me", self.api_base_url)
            .parse::<Url>()
            .context("failed to parse API base URL")?;
        // self.cookie_store.add_cookie_str(cookie, &url);
        let resp = self
            .client
            .get(url)
            .header(COOKIE, format!("{}={}", CAMPSITE_API_COOKIE, cookie_value))
            .send()
            .await?;

        if resp.status().is_success() {
            // let text = resp.text().await?;
            // println!("Raw response: {}", text);
            let campsite_user = resp.json::<CampsiteUserJson>().await?;
            let login_user: LoginUser = campsite_user.into();
            session
                .insert("user", &login_user)
                .context("failed in inserting serialized value into session")?;
            Ok(Some(session))
        } else {
            tracing::error!("load session Status: {}", resp.status());
            Ok(None)
        }
    }

    async fn store_session(&self, _: Session) -> Result<Option<String>> {
        Err(anyhow::anyhow!("store_session is not supported"))
    }

    async fn destroy_session(&self, _: Session) -> Result {
        Err(anyhow::anyhow!("destroy_session is not supported"))
    }

    async fn clear_store(&self) -> Result {
        Err(anyhow::anyhow!("clear_store is not supported"))
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
}
