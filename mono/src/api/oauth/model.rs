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
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CampsiteUserJson {
    pub username: String,
    pub id: String,
    pub avatar_url: String,
    pub email: Option<String>,
}

impl From<CampsiteUserJson> for LoginUser {
    fn from(value: CampsiteUserJson) -> Self {
        Self {
            username: value.username,
            email: value.email.unwrap_or_default(),
            avatar_url: value.avatar_url,
            campsite_user_id: value.id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LoginUser {
    pub campsite_user_id: String,
    pub username: String,
    pub avatar_url: String,
    pub email: String,
}
