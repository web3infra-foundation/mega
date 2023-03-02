//!
//!
//!
//!

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum GitError {
    #[error("The `{0}` is not a valid git object type.")]
    InvalidObjectType(String),

    #[error("The `{0}` is not a valid git blob object.")]
    InvalidBlobObject(String),

    #[error("The `{0}` is not a valid git tree object.")]
    InvalidTreeObject(String),

    #[error("The `{0}` is not a valid git tree item.")]
    InvalidTreeItem(String),

    #[error("The `{0}` is not a valid git commit object.")]
    InvalidCommitObject(String),

    #[error("The `{0}` is not a valid git tag object.")]
    InvalidTagObject(String),

    #[error("The `{0}` is not a valid idx file.")]
    InvalidIdxFile(String),

    #[error("The `{0}` is not a valid pack file.")]
    InvalidPackFile(String),

    #[error("The `{0}` is not a valid pack header.")]
    InvalidPackHeader(String),

    #[error("The {0} is not a valid Hash value ")]
    InvalidHashValue(String),

    #[error("Delta Object Error Info:{0}")]
    DeltaObjError(String),

    #[error("The object to be packed is incomplete ,{0}")]
    UnCompletedPackObject(String),

    #[error("Error decode in the Object ,info:{0}")]
    InvalidObjectInfo(String),

    #[error("Can't found Hash value :{0} from current file")]
    NotFountHashValue(String),
}