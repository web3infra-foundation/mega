use common::errors::ProtocolError;
use thiserror::Error;

pub type MonoBeanResult<T> = Result<T, MonoBeanError>;

#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum MonoBeanError {
    #[error("Mega Core Error: {0}")]
    MegaCoreError(String),
    
    #[error("Mega Protocol Error: {0}")]
    MegaProtocolError(#[from] ProtocolError),
    
    #[error("Mega Server Error: {0}")]
    MegaServerError(#[from] std::io::Error),
}