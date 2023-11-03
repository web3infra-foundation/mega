//!
//!
//!
//!
//!
use thiserror::Error;

use anyhow::Result;

pub type MegaResult = Result<(), MegaError>;

///
///
#[derive(Debug)]
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

    pub fn unknown_subcommand(cmd: &str) -> MegaError {
        MegaError {
            error: anyhow::anyhow!("Unknown subcommand: {}", cmd).into(),
            code: 1,
        }
    }

    pub fn with_message(msg: &str) -> MegaError {
        MegaError {
            error: anyhow::anyhow!("Error Message: {}", msg).into(),
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

#[derive(Error, Debug)]
#[allow(unused)]
pub enum GitLFSError {
    #[error("Something went wrong in Git LFS")]
    GeneralError(String),
}

#[cfg(test)]
mod tests {}
