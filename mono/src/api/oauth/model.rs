use chrono::{DateTime, NaiveDateTime, Utc};
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CampsiteUserJson {
    pub username: String,
    pub id: String,
    pub avatar_url: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<CampsiteUserJson> for LoginUser {
    fn from(value: CampsiteUserJson) -> Self {
        Self {
            username: value.username,
            email: value.email.unwrap_or_default(),
            avatar_url: value.avatar_url,
            campsite_user_id: value.id,
            created_at: value.created_at.naive_utc(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginUser {
    pub campsite_user_id: String,
    pub username: String,
    pub avatar_url: String,
    pub email: String,
    pub created_at: NaiveDateTime,
}

impl From<user::Model> for LoginUser {
    fn from(value: user::Model) -> Self {
        Self {
            avatar_url: value.avatar_url,
            email: value.email,
            created_at: value.created_at,
            campsite_user_id: String::new(),
            username: String::new()
        }
    }
}
