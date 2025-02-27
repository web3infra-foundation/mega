use callisto::{access_token, ssh_keys};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AddSSHKey {
    pub title: String,
    pub ssh_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSSHKey {
    pub id: i64,
    pub title: String,
    pub ssh_key: String,
    pub finger: String,
    pub created_at: NaiveDateTime,
}

impl From<ssh_keys::Model> for ListSSHKey {
    fn from(value: ssh_keys::Model) -> Self {
        Self {
            id: value.id,
            title: value.title,
            ssh_key: value.ssh_key,
            finger: value.finger,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToken {
    pub id: i64,
    pub token: String,
    pub created_at: NaiveDateTime,
}

impl From<access_token::Model> for ListToken {
    fn from(value: access_token::Model) -> Self {
        let mut mask_token = value.token;
        mask_token.replace_range(7..32, "-******-");
        Self {
            id: value.id,
            token: mask_token,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoPermissions {
    pub admin: Vec<String>,
    pub maintainer: Vec<String>,
    pub reader: Vec<String>,
}
