use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthorizeParams {
    pub redirect_uri: String,
}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GitHubUserJson {
    pub login: String,
    pub id: u32,
    pub avatar_url: String,
    pub email: String,
}
