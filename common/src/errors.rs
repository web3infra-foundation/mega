use std::convert::Infallible;

use anyhow::Result;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cedar_policy::ParseErrors;
use git_internal::errors::GitError;
use thiserror::Error;

use crate::model::CommonResult;

pub type MegaResult = Result<(), MegaError>;

#[derive(Error, Debug)]
pub enum MegaError {
    // --- Redis ---
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Bincode encode error: {0}")]
    EncodeError(#[from] bincode::error::EncodeError),

    // --- Serialization / parsing ---
    #[error("JSON serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    // --- IO ---
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // --- Database ---
    #[error("Database error: {0}")]
    Db(#[from] sea_orm::DbErr),

    // --- PGP ---
    #[error("PGP error: {0}")]
    Pgp(#[from] Box<pgp::errors::Error>),

    // --- Clap ---
    #[error("Clap error: {0}")]
    Clap(#[from] clap::Error),

    // --- Anyhow ---
    #[error("Generic error: {0}")]
    Anyhow(#[from] anyhow::Error),

    // --- GitError ---
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    // --- BuckError ---
    #[error("Buck API error: {0}")]
    Buck(#[from] BuckError),

    #[error("Not Found error: {0}")]
    NotFound(String),

    #[error("ObjStorage error: {0}")]
    ObjStorage(String),

    // --- Other ---
    #[error("Other error: {0}")]
    Other(String),
}

impl From<Infallible> for MegaError {
    fn from(err: Infallible) -> MegaError {
        match err {}
    }
}

impl From<ParseErrors> for MegaError {
    fn from(err: ParseErrors) -> MegaError {
        MegaError::Other(err.to_string())
    }
}

impl From<MegaError> for GitError {
    fn from(val: MegaError) -> Self {
        GitError::CustomError(val.to_string())
    }
}

#[derive(Error, Debug)]
pub enum GitLFSError {
    #[error("Something went wrong in Git LFS: {0}")]
    GeneralError(String),
}

/// Buck upload API errors
#[derive(Debug, Error)]
pub enum BuckError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session expired")]
    SessionExpired,

    #[error("File not in manifest: {0}")]
    FileNotInManifest(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("File size exceeds limit: {0} > {1}")]
    FileSizeExceedsLimit(u64, u64),

    #[error("File already uploaded: {0}")]
    FileAlreadyUploaded(String),

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Invalid session status: expected {expected:?}, got {actual:?}")]
    InvalidSessionStatus { expected: String, actual: String },

    #[error("Files not fully uploaded: {missing_count} files remaining")]
    FilesNotFullyUploaded { missing_count: u32 },
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("Authentication failed: {0}")]
    Deny(String),
    #[error("Repository not found: {0}")]
    NotFound(String),
    #[error("PackFile too large: {0}")]
    TooLarge(String),
    #[error("Invalid Input: {0}")]
    InvalidInput(String),
    #[error("HTTP Push Has Been Disabled")]
    Disabled,
}

impl From<MegaError> for ProtocolError {
    fn from(err: MegaError) -> ProtocolError {
        ProtocolError::InvalidInput(err.to_string())
    }
}

impl IntoResponse for ProtocolError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ProtocolError::Deny(err) => {
                // This error is caused by bad user input so don't log it
                (StatusCode::UNAUTHORIZED, err)
            }
            ProtocolError::TooLarge(err) => (StatusCode::PAYLOAD_TOO_LARGE, err),
            ProtocolError::NotFound(err) => {
                // Because `TraceLayer` wraps each request in a span that contains the request
                // method, uri, etc we don't need to include those details here
                // tracing::error!(%err, "error");

                // Don't expose any details about the error to the client
                (StatusCode::NOT_FOUND, err)
            }
            ProtocolError::InvalidInput(err) => (StatusCode::BAD_REQUEST, err),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong".to_owned(),
            ),
        };

        (status, Json(CommonResult::<String>::failed(&message))).into_response()
    }
}

#[cfg(test)]
mod tests {}
