use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum TransferMode {
    #[default]
    BASIC,
    MULTIPART,
    //not implement yet
    STREAMING,
}

#[derive(Debug, Default)]
pub struct MetaObject {
    pub oid: String,
    pub size: i64,
    pub exist: bool,
    pub splited: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RequestVars {
    pub oid: String,
    pub size: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repo: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub authorization: String,
}

impl RequestVars {
    pub async fn download_link(&self, ext_origin: String) -> String {
        self.internal_link("objects".to_string(), ext_origin).await
    }

    pub async fn upload_link(&self, ext_origin: String) -> String {
        self.internal_link("objects".to_string(), ext_origin).await
    }

    async fn internal_link(&self, subpath: String, ext_origin: String) -> String {
        let mut path = PathBuf::new();

        let user = &self.user;
        if !user.is_empty() {
            path.push(user);
        }

        let repo = &self.repo;
        if !repo.is_empty() {
            path.push(repo);
        }

        path.push(ext_origin);

        path.push(&subpath);
        path.push(&self.oid);

        path.into_os_string().into_string().unwrap()
    }

    pub async fn verify_link(&self, ext_origin: String) -> String {
        let path = format!("/verify/{}", &self.oid);
        format!("{}{}", ext_origin, path)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lock {
    pub id: String,
    pub path: String,
    pub locked_at: String,
    pub owner: Option<User>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct User {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct BatchRequest {
    pub operation: String,
    pub transfers: Vec<String>,
    pub objects: Vec<RequestVars>,
    pub hash_algo: String,
}

#[derive(Serialize, Deserialize)]
pub struct BatchResponse {
    pub transfer: String,
    pub objects: Vec<Representation>,
    pub hash_algo: String,
}

#[derive(Serialize, Deserialize)]
pub struct FetchchunkResponse {
    pub oid: String,
    pub size: i64,
    pub chunks: Vec<ChunkRepresentation>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Link {
    pub href: String,
    #[serde(default)] // Optional field
    pub header: HashMap<String, String>,
    pub expires_at: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ObjectError {
    pub code: i64,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct Representation {
    pub oid: String,
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<HashMap<String, Link>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ObjectError>,
}

#[derive(Serialize, Deserialize)]
pub struct ChunkRepresentation {
    pub sub_oid: String,
    pub offset: i64,
    pub size: i64,
    pub link: Link,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Ref {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LockRequest {
    pub path: String,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub refs: Ref,
}

#[derive(Serialize, Deserialize)]
pub struct LockResponse {
    pub lock: Lock,
    pub message: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct UnlockRequest {
    pub force: Option<bool>,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub refs: Ref,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnlockResponse {
    pub lock: Lock,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct LockList {
    pub locks: Vec<Lock>,
    pub next_cursor: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct VerifiableLockRequest {
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub refs: Ref,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifiableLockList {
    pub ours: Vec<Lock>,
    pub theirs: Vec<Lock>,
    pub next_cursor: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LockListQuery {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub cursor: String,
    #[serde(default)]
    pub limit: String,
    #[serde(default)]
    pub refspec: String,
}
