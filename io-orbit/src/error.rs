use common::errors::MegaError;

#[derive(Debug, thiserror::Error)]
pub enum IoOrbitError {
    #[error("object store error: {0}")]
    ObjectStore(#[from] object_store::Error),

    #[error("write manifest precondition failed")]
    WriteManifestPreconditionFailed,

    #[error("other error: {0}")]
    Other(#[from] MegaError),
}

impl From<IoOrbitError> for MegaError {
    fn from(err: IoOrbitError) -> Self {
        match err {
            IoOrbitError::ObjectStore(e) => MegaError::ObjStorage(e.to_string()),
            IoOrbitError::WriteManifestPreconditionFailed => {
                MegaError::Other("write manifest precondition failed".to_string())
            }
            IoOrbitError::Other(e) => e,
        }
    }
}
