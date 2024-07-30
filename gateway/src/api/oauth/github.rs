use axum::async_trait;

use common::errors::MegaError;
use jupiter::context::Context;

use crate::api::oauth::model::{AuthorizeParams, GitHubAccessTokenJson, OauthCallbackParams};
use crate::api::oauth::OauthHandler;

use super::model::GitHubUserJson;

#[derive(Clone)]
pub struct GithubOauthService {
    pub context: Context,
    pub client_id: String,
    pub client_secret: String,
}

const GITHUB_ENDPOINT: &str = "https://github.com";
const GITHUB_API_ENDPOINT: &str = "https://api.github.com";

#[async_trait]
impl OauthHandler for GithubOauthService {
    fn authorize_url(&self, params: &AuthorizeParams, state: &str) -> String {
        let auth_url = format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}",
            self.client_id, params.redirect_uri, state
        );
        auth_url
    }

    async fn access_token(
        &self,
        params: OauthCallbackParams,
        redirect_uri: &str,
    ) -> Result<String, MegaError> {
        tracing::debug!("{:?}", params);
        // get access_token and user for persist
        let url = format!(
            "{}/login/oauth/access_token?client_id={}&client_secret={}&code={}&redirect_uri={}",
            GITHUB_ENDPOINT, self.client_id, self.client_secret, params.code, redirect_uri
        );
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header("Accept", "application/json")
            .send()
            .await
            .unwrap();
        let access_token = resp
            .json::<GitHubAccessTokenJson>()
            .await
            .unwrap()
            .access_token;
        Ok(access_token)
    }

    async fn user_info(&self, access_token: &str) -> Result<GitHubUserJson, MegaError> {
        let user_url = format!("{}/user", GITHUB_API_ENDPOINT);
        let client = reqwest::Client::new();
        let resp = client
            .get(user_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .header("User-Agent", format!("Mega/{}", "0.0.1"))
            .send()
            .await
            .unwrap();
        let mut user_info = GitHubUserJson::default();
        if resp.status().is_success() {
            user_info = resp.json::<GitHubUserJson>().await.unwrap();
        } else {
            tracing::error!("github:user_info:err {:?}", resp.text().await.unwrap());
        }
        Ok(user_info)
    }
}
