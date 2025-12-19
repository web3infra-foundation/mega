use callisto::lfs_objects;
use callisto::sea_orm_active_enums::StorageTypeEnum;
use chrono::{DateTime, Duration, Utc};
use common::config::LFSConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
pub enum TransferMode {
    #[default]
    #[serde(rename = "basic")]
    BASIC,
    #[serde(rename = "multipart")]
    MULTIPART,
    //not implement yet
    STREAMING,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone, ToSchema)]
pub enum Operation {
    #[serde(rename = "download")]
    Download,
    #[serde(rename = "upload")]
    Upload,
}

/// Download operations MUST specify a download action, or an object error if the object cannot be downloaded for some reason.
/// Upload operations can specify an upload and a verify action.
/// The upload action describes how to upload the object. If the object has a verify action, the LFS client will hit this URL after a successful upload. Servers can use this for extra verification, if needed.
/// If a client requests to upload an object that the server already has, the server should omit the actions property completely. The client will then assume the server already has it.
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, ToSchema)]
pub enum Action {
    #[serde(rename = "download")]
    Download,
    #[serde(rename = "upload")]
    Upload,
    #[serde(rename = "verify")]
    Verify,
}

#[derive(Debug, Clone)]
pub struct MetaObject {
    pub oid: String,
    pub size: i64,
    pub exist: bool,
    pub splited: bool,
}

impl From<lfs_objects::Model> for MetaObject {
    fn from(value: lfs_objects::Model) -> Self {
        Self {
            oid: value.oid,
            size: value.size,
            exist: value.exist,
            splited: value.splited,
        }
    }
}

impl From<MetaObject> for lfs_objects::Model {
    fn from(value: MetaObject) -> Self {
        Self {
            oid: value.oid,
            size: value.size,
            exist: value.exist,
            splited: value.splited,
        }
    }
}

impl MetaObject {
    pub fn new(req_obj: &RequestObject, config: &LFSConfig) -> Self {
        let splited = config.local.enable_split;
        Self {
            oid: req_obj.oid.to_string(),
            size: req_obj.size,
            exist: true,
            splited: if StorageTypeEnum::AwsS3 == config.storage_type.clone().into() {
                false
            } else {
                splited
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
/// Request object for LFS operations
pub struct RequestObject {
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

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
/// LFS lock information
pub struct Lock {
    pub id: String,
    pub path: String,
    pub locked_at: String,
    pub owner: Option<User>,
}

/// User information for lock ownership
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, ToSchema)]
pub struct User {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
/// Batch request for LFS operations
pub struct BatchRequest {
    // Should be download or upload.
    pub operation: Operation,
    // An optional Array of String identifiers for transfer adapters that the client has configured.
    // If omitted, the basic transfer adapter MUST be assumed by the server.
    pub transfers: Vec<String>,
    pub objects: Vec<RequestObject>,
    pub hash_algo: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
/// Batch response for LFS operations
pub struct BatchResponse {
    pub transfer: TransferMode,
    pub objects: Vec<ResponseObject>,
    pub hash_algo: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
/// Response for fetching chunk IDs
pub struct FetchChunkResponse {
    pub oid: String,
    pub size: i64,
    pub chunks: Vec<ChunkDownloadObject>,
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
/// Link information for LFS object transfer
pub struct Link {
    pub href: String,
    #[serde(default)] // Optional field
    pub header: HashMap<String, String>,
    pub expires_at: String,
}

impl Link {
    pub fn new(href: &str) -> Self {
        let mut header = HashMap::new();
        header.insert("Accept".to_string(), "application/vnd.git-lfs".to_owned());

        Link {
            href: href.to_string(),
            header,
            expires_at: {
                let expire_time: DateTime<Utc> = Utc::now() + Duration::try_seconds(86400).unwrap();
                expire_time.to_rfc3339()
            },
        }
    }
}

#[derive(Serialize, Deserialize, Default, ToSchema)]
/// Error information for LFS object operations
pub struct ObjectError {
    pub code: i64,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
/// Response object for LFS batch operations
pub struct ResponseObject {
    pub oid: String,
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticated: Option<bool>,
    // Object containing the next actions for this object. Applicable actions depend on which operation is specified in the request.
    // How these properties are interpreted depends on which transfer adapter the client will be using.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<HashMap<Action, Link>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ObjectError>,
}

pub struct ResCondition {
    pub file_exist: bool,
    pub operation: Operation,
    pub use_tus: bool,
}

impl ResponseObject {
    pub fn new(
        meta: &MetaObject,
        res_condition: ResCondition,
        download_url: &str,
        upload_url: &str,
    ) -> ResponseObject {
        let mut res = ResponseObject {
            oid: meta.oid.to_owned(),
            size: meta.size,
            authenticated: Some(true),
            actions: None,
            error: None,
        };

        let mut actions = HashMap::new();

        match res_condition {
            ResCondition {
                file_exist: true,
                operation: Operation::Upload,
                ..
            } => {
                //If a client requests to upload an object that the server already has, the server should omit the actions property completely.
                // The client will then assume the server already has it.
                tracing::debug!("File existing, leave actions empty")
            }
            ResCondition {
                file_exist: true,
                operation: Operation::Download,
                ..
            } => {
                actions.insert(Action::Download, Link::new(download_url));
                res.actions = Some(actions);
            }
            ResCondition {
                file_exist: false,
                operation: Operation::Upload,
                ..
            } => {
                actions.insert(Action::Upload, Link::new(upload_url));
                // if use_tus {
                //     actions.insert(
                //         Action::Verify,
                //         Link::new(&req_object.verify_link(hostname.to_string())),
                //     );
                // }
                res.actions = Some(actions);
            }
            ResCondition {
                file_exist: false,
                operation: Operation::Download,
                ..
            } => {
                let err = ObjectError {
                    code: 404,
                    message: "Not found".to_owned(),
                };
                res.error = Some(err)
            }
        }
        res
    }

    pub fn failed_with_err(object: &RequestObject, err: ObjectError) -> ResponseObject {
        ResponseObject {
            oid: object.oid.to_owned(),
            size: object.size,
            authenticated: None,
            actions: None,
            error: Some(err),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
/// Chunk download object information
pub struct ChunkDownloadObject {
    pub sub_oid: String,
    pub offset: i64,
    pub size: i64,
    pub link: Link,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, ToSchema)]
/// Git reference information
pub struct Ref {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
/// Request to create a lock
pub struct LockRequest {
    pub path: String,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub refs: Ref,
}

#[derive(Serialize, Deserialize, ToSchema)]
/// Response after creating a lock
pub struct LockResponse {
    pub lock: Lock,
    pub message: String,
}

#[derive(Serialize, Deserialize, Default, ToSchema)]
/// Request to unlock a file
pub struct UnlockRequest {
    pub force: Option<bool>,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub refs: Ref,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
/// Response after unlocking a file
pub struct UnlockResponse {
    pub lock: Lock,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
/// List of locks
pub struct LockList {
    pub locks: Vec<Lock>,
    pub next_cursor: String,
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
/// Request to verify locks
pub struct VerifiableLockRequest {
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    pub refs: Ref,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
/// List of verifiable locks
pub struct VerifiableLockList {
    pub ours: Vec<Lock>,
    pub theirs: Vec<Lock>,
    pub next_cursor: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
/// Query parameters for listing locks
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
