use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

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

impl From<GitHubUserJson> for LoginUser {
    fn from(value: GitHubUserJson) -> Self {
        Self {
            username: value.login,
            email: value.email.unwrap_or_default(),
            avatar_url: value.avatar_url,
            campsite_user_id: String::new(),
            created_at: Utc::now().naive_utc(),
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
