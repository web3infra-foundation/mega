use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error occurs in mega: {0:?}")]
    MegaError(common::errors::MegaError),

    #[error("Error loading assets: {0}")]
    AssetsError(String),
}