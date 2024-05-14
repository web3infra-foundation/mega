use thiserror::Error;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum GitDeltaError{
    #[error("The `{0}` is not a valid git object type.")]
    DeltaEncoderError(String),

    #[error("The `{0}` is not a valid git object type.")]
    DeltaDecoderError(String),
}