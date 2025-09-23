use axum::response::{IntoResponse, Json, Response};
use common::model::CommonResult;
use http::StatusCode;

#[derive(Debug)]
pub struct ApiError {
    inner: anyhow::Error,
    status: StatusCode,
}

impl ApiError {
    // Preserve an API that can be constructed from an anyhow::Error (maps to 500 by default)
    pub fn new(err: impl Into<anyhow::Error>) -> Self {
        Self {
            inner: err.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    // Create an ApiError with explicit status code
    pub fn with_status(status: StatusCode, err: impl Into<anyhow::Error>) -> Self {
        Self {
            inner: err.into(),
            status,
        }
    }

    pub fn bad_request(err: impl Into<anyhow::Error>) -> Self {
        Self::with_status(StatusCode::BAD_REQUEST, err)
    }

    pub fn not_found(err: impl Into<anyhow::Error>) -> Self {
        Self::with_status(StatusCode::NOT_FOUND, err)
    }

    pub fn internal(err: impl Into<anyhow::Error>) -> Self {
        Self::with_status(StatusCode::INTERNAL_SERVER_ERROR, err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!("Application error: {:#}", self.inner);

        let body = Json(CommonResult::<()>::common_failed());
        (self.status, body).into_response()
    }
}

// Backwards-compatible From: map generic errors to ApiError::internal
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        ApiError::internal(err)
    }
}
