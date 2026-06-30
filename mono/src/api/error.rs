use api_model::common::CommonResult;
use axum::response::{IntoResponse, Json, Response};
use common::errors::{
    MegaError, mega_error_http_status, mega_error_is_client_safe, parse_legacy_http_code,
};
use http::StatusCode;

fn status_code_from_u16(code: u16) -> StatusCode {
    StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Debug)]
pub struct ApiError {
    inner: anyhow::Error,
    status: StatusCode,
}

impl ApiError {
    pub fn new(err: impl Into<anyhow::Error>) -> Self {
        Self {
            inner: err.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

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

    #[cfg(test)]
    fn status_code(&self) -> StatusCode {
        self.status
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let err_str = self.inner.to_string();

        let err_msg = if let Some((_, msg)) = parse_legacy_http_code(&err_str) {
            msg.to_string()
        } else {
            err_str
        };

        if self.status.is_client_error() {
            tracing::warn!(status = %self.status, "Client error: {}", err_msg);
        } else {
            tracing::error!(status = %self.status, "Application error: {}", err_msg);
        }

        let response_msg = if self.status.is_client_error() {
            err_msg
        } else {
            "Internal server error".to_string()
        };

        let body = Json(CommonResult::<()> {
            req_result: false,
            data: None,
            err_message: response_msg,
        });
        (self.status, body).into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        let anyhow_err = err.into();

        if let Some(mega_err) = anyhow_err.downcast_ref::<MegaError>() {
            let status = status_code_from_u16(mega_error_http_status(mega_err));
            if !mega_error_is_client_safe(mega_err) {
                tracing::error!(error_type = ?mega_err, "Internal error occurred");
                tracing::debug!("Internal error: {:?}", mega_err);
                return ApiError::internal(anyhow::anyhow!("Internal server error"));
            }
            return ApiError::with_status(status, anyhow_err);
        }

        let err_str = anyhow_err.to_string();
        if let Some((code, _)) = parse_legacy_http_code(&err_str) {
            let status = status_code_from_u16(code);
            if status.is_server_error() {
                return ApiError::internal(anyhow_err);
            }
            return ApiError::with_status(status, anyhow_err);
        }

        ApiError::internal(anyhow_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mega_not_found_maps_to_404() {
        let api_err = ApiError::from(MegaError::NotFound("CL not found".into()));
        assert_eq!(api_err.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn mega_unavailable_maps_to_503() {
        let api_err = ApiError::from(MegaError::Unavailable("Build system is not enabled".into()));
        assert_eq!(api_err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn legacy_other_code_maps_to_503() {
        let api_err = ApiError::from(MegaError::Other(
            "[code:503] Build system is not enabled".into(),
        ));
        assert_eq!(api_err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn git_wrapped_not_found_maps_to_404() {
        use git_internal::errors::GitError;
        let api_err = ApiError::from(MegaError::Git(GitError::CustomError(
            "File not found".into(),
        )));
        assert_eq!(api_err.status_code(), StatusCode::NOT_FOUND);
    }
}
