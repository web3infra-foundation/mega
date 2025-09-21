use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("The `{0}` is not a valid git object type.")]
    InvalidObjectType(String),

    #[error("The `{0}` is not a valid git blob object.")]
    InvalidBlobObject(String),

    #[error("Not a valid git tree object.")]
    InvalidTreeObject,

    #[error("The `{0}` is not a valid git tree item.")]
    InvalidTreeItem(String),

    #[error("`{0}`.")]
    EmptyTreeItems(String),

    #[error("The `{0}` is not a valid git commit signature.")]
    InvalidSignatureType(String),

    #[error("Not a valid git commit object.")]
    InvalidCommitObject,

    #[error("Invalid Commit: {0}")]
    InvalidCommit(String),

    #[error("Not a valid git tag object: {0}")]
    InvalidTagObject(String),

    #[error("The `{0}` is not a valid idx file.")]
    InvalidIdxFile(String),

    #[error("The `{0}` is not a valid pack file.")]
    InvalidPackFile(String),

    #[error("The `{0}` is not a valid pack header.")]
    InvalidPackHeader(String),

    #[error("The `{0}` is not a valid index file.")]
    InvalidIndexFile(String),

    #[error("The `{0}` is not a valid index header.")]
    InvalidIndexHeader(String),

    #[error("Argument parse failed: {0}")]
    InvalidArgument(String),

    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("The {0} is not a valid Hash value ")]
    InvalidHashValue(String),

    #[error("Delta Object Error Info:{0}")]
    DeltaObjectError(String),

    #[error("The object to be packed is incomplete ,{0}")]
    UnCompletedPackObject(String),

    #[error("Error decode in the Object ,info:{0}")]
    InvalidObjectInfo(String),

    #[error("Can't found Hash value :{0} from current file")]
    NotFountHashValue(String),

    #[error("Can't encode the object which id [{0}] to bytes")]
    EncodeObjectError(String),

    #[error("UTF-8 conversion error: {0}")]
    ConversionError(String),

    #[error("Can't find parent tree by path: {0}")]
    InvalidPathError(String),

    #[error("Can't encode entries to pack: {0}")]
    PackEncodeError(String),

    #[error("Can't find specific object: {0}")]
    ObjectNotFound(String),

    #[error("Repository not found")]
    RepoNotFound,

    #[error("UnAuthorized: {0}")]
    UnAuthorized(String),

    #[error("Network Error: {0}")]
    NetworkError(String),

    #[error("{0}")]
    CustomError(String),
}

impl From<FromUtf8Error> for GitError {
    fn from(err: FromUtf8Error) -> Self {
        // convert the FromUtf8Error to GitError and return it
        GitError::ConversionError(err.to_string())
    }
}
