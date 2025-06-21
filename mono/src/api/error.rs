use axum::response::{IntoResponse, Json, Response};
use common::model::CommonResult;
use http::StatusCode;

#[derive(Debug)]
pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!("Application error: {:#}", self.0);

        let body = Json(CommonResult::<()>::common_failed());
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, ApiError>`. That way you don't need to do that manually.
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
