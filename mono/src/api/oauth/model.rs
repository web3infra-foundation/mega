use serde::{Deserialize, Serialize};

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

/// Tinyship / better-auth `GET /api/auth/get-session` response body.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TinyshipGetSessionResponse {
    pub session: Option<TinyshipSessionJson>,
    pub user: Option<TinyshipAuthUserJson>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TinyshipSessionJson {
    pub id: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TinyshipAuthUserJson {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub image: Option<String>,
}

impl From<TinyshipAuthUserJson> for LoginUser {
    fn from(value: TinyshipAuthUserJson) -> Self {
        Self {
            campsite_user_id: value.id,
            username: value.name,
            email: value.email.unwrap_or_default(),
            avatar_url: value.image.unwrap_or_default(),
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
