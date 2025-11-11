use anyhow::Result;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use git_internal::errors::GitError;
use thiserror::Error;

use crate::model::CommonResult;

pub type MegaResult = Result<(), MegaError>;

#[derive(Error, Debug)]
pub struct MegaError {
    pub error: Option<anyhow::Error>,
    pub code: i32,
}

impl MegaError {
    pub fn new(error: anyhow::Error, code: i32) -> MegaError {
        MegaError {
            error: Some(error),
            code,
        }
    }

    pub fn print(&self) {
        panic!("{}", self.error.as_ref().unwrap());
    }

    pub fn unknown_subcommand(cmd: impl AsRef<str>) -> MegaError {
        MegaError {
            error: anyhow::anyhow!("Unknown subcommand: {}", cmd.as_ref()).into(),
            code: 1,
        }
    }

    pub fn with_message(msg: impl AsRef<str>) -> MegaError {
        MegaError {
            error: anyhow::anyhow!("Error Message: {}", msg.as_ref()).into(),
            code: 0,
        }
    }
}

impl std::fmt::Display for MegaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.error.as_ref().unwrap())
    }
}

impl From<anyhow::Error> for MegaError {
    fn from(err: anyhow::Error) -> MegaError {
        MegaError::new(err, 101)
    }
}

impl From<clap::Error> for MegaError {
    fn from(err: clap::Error) -> MegaError {
        let code = i32::from(err.use_stderr());
        MegaError::new(err.into(), code)
    }
}

impl From<std::io::Error> for MegaError {
    fn from(err: std::io::Error) -> MegaError {
        MegaError::new(err.into(), 1)
    }
}

impl From<sea_orm::DbErr> for MegaError {
    fn from(err: sea_orm::DbErr) -> MegaError {
        MegaError::new(err.into(), 1)
    }
}

impl From<pgp::errors::Error> for MegaError {
    fn from(err: pgp::errors::Error) -> MegaError {
        MegaError::new(err.into(), 1)
    }
}

impl From<MegaError> for GitError {
    fn from(val: MegaError) -> Self {
        GitError::CustomError(val.to_string())
    }
}

impl From<GitError> for MegaError {
    fn from(val: GitError) -> Self {
        MegaError::with_message(val.to_string())
    }
}

#[derive(Error, Debug)]
pub enum GitLFSError {
    #[error("Something went wrong in Git LFS: {0}")]
    GeneralError(String),
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
        ProtocolError::InvalidInput(err.error.unwrap().to_string())
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
