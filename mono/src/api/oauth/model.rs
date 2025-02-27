use chrono::NaiveDateTime;
use common::utils::generate_id;
use serde::{Deserialize, Serialize};

use callisto::user;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OauthCallbackParams {
    pub code: String,
    pub state: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GitHubAccessTokenJson {
    pub access_token: String,
    pub scope: Option<String>,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GitHubUserJson {
    pub login: String,
    pub id: u32,
    pub avatar_url: String,
    // email can be null from github
    pub email: Option<String>,
}

impl From<GitHubUserJson> for user::Model {
    fn from(value: GitHubUserJson) -> Self {
        Self {
            id: generate_id(),
            name: value.login,
            email: value.email.unwrap_or_default(),
            avatar_url: value.avatar_url,
            is_github: true,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginUser {
    pub user_id: i64,
    pub name: String,
    pub avatar_url: String,
    pub email: String,
    pub created_at: NaiveDateTime,
}

impl From<user::Model> for LoginUser {
    fn from(value: user::Model) -> Self {
        Self {
            user_id: value.id,
            name: value.name,
            avatar_url: value.avatar_url,
            email: value.email,
            created_at: value.created_at,
        }
    }
}
