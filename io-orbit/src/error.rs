use common::errors::MegaError;

#[derive(Debug, thiserror::Error)]
pub enum IoOrbitError {
    #[error("object store error: {0}")]
    ObjectStore(#[from] object_store::Error),
}

impl From<IoOrbitError> for MegaError {
    fn from(err: IoOrbitError) -> Self {
        match err {
            IoOrbitError::ObjectStore(e) => MegaError::ObjStorage(e.to_string()),
        }
    }
}
