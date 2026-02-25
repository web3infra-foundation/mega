use api_model::common::CommonResult;
use axum::response::{IntoResponse, Json, Response};
use common::errors::{BuckError, MegaError};
use http::StatusCode;

/// Parse [code:xxx] format from error message.
/// Returns (status_code, clean_message) if found, None otherwise.
fn parse_error_code(err_str: &str) -> Option<(&str, &str)> {
    // Find [code:xxx] anywhere in the string
    let start = err_str.find("[code:")?;
    let code_start = start + 6; // Skip "[code:"

    // Find the closing bracket after [code:
    let remaining = &err_str[start..];
    let code_end_relative = remaining.find(']')?;

    // Ensure we have at least one character for the code
    if code_end_relative <= 6 {
        return None;
    }

    let code_end = start + code_end_relative;
    let code = &err_str[code_start..code_end];

    // Validate that code is not empty and contains only valid characters
    if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    // Extract message after the closing bracket; allow empty messages for compatibility
    let msg_start = code_end + 1;
    let msg = err_str.get(msg_start..).unwrap_or("").trim_start();

    Some((code, msg))
}

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
        let err_str = self.inner.to_string();

        // Remove [code:xxx] prefix from error message for cleaner display
        let err_msg = if let Some((_, msg)) = parse_error_code(&err_str) {
            msg.to_string()
        } else {
            err_str
        };

        tracing::error!("Application error: {}", err_msg);

        // Only expose detailed error messages for 4xx (client) errors
        // For 5xx (server) errors, use generic message to avoid leaking internal details
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

// Generic From implementation: converts any error to ApiError
// 1. Typed MegaError matching (for type-safe error handling)
// 2. Fallback: parse [code:xxx] format (for backwards compatibility)
// 3. Default: internal server error
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        let anyhow_err = err.into();

        // Try typed matching first: check if error is MegaError
        // Use downcast_ref (borrowing) instead of downcast (ownership) to preserve error context
        if let Some(mega_err) = anyhow_err.downcast_ref::<MegaError>() {
            // Handle Buck business errors with specific HTTP status codes
            if let MegaError::Buck(buck_err) = mega_err {
                let status = match buck_err {
                    BuckError::SessionNotFound(_) | BuckError::FileNotInManifest(_) => {
                        StatusCode::NOT_FOUND
                    }
                    BuckError::SessionExpired => StatusCode::GONE,
                    BuckError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
                    BuckError::FileSizeExceedsLimit(_, _) => StatusCode::PAYLOAD_TOO_LARGE,
                    BuckError::FileAlreadyUploaded(_) => StatusCode::CONFLICT,
                    BuckError::Forbidden(_) => StatusCode::FORBIDDEN,
                    BuckError::HashMismatch { .. }
                    | BuckError::ValidationError(_)
                    | BuckError::InvalidSessionStatus { .. }
                    | BuckError::FilesNotFullyUploaded { .. } => StatusCode::BAD_REQUEST,
                };
                // Use original anyhow_err to preserve stack trace
                return ApiError::with_status(status, anyhow_err);
            }

            // Handle other MegaError variants
            match mega_err {
                MegaError::NotFound(_) => return ApiError::not_found(anyhow_err),
                MegaError::Db(_) | MegaError::Redis(_) | MegaError::Io(_) => {
                    // Hide internal details in production, return generic 500
                    tracing::error!(
                        error_type = %match mega_err {
                            MegaError::Db(_) => "Db",
                            MegaError::Redis(_) => "Redis",
                            MegaError::Io(_) => "Io",
                            _ => "Other",
                        },
                        "Internal error occurred"
                    );
                    tracing::debug!("Internal error: {:?}", mega_err);
                    return ApiError::internal(anyhow::anyhow!("Internal server error"));
                }
                // For other MegaError variants, fall through to parse [code:xxx] format
                _ => {}
            }
        }

        // Fallback: parse [code:xxx] format to set proper HTTP status code
        // This handles legacy error format and non-MegaError types
        let err_str = anyhow_err.to_string();
        if let Some((code, _)) = parse_error_code(&err_str) {
            return match code {
                "400" => ApiError::bad_request(anyhow_err),
                "401" => ApiError::with_status(StatusCode::UNAUTHORIZED, anyhow_err),
                "403" => ApiError::with_status(StatusCode::FORBIDDEN, anyhow_err),
                "404" => ApiError::not_found(anyhow_err),
                "409" => ApiError::with_status(StatusCode::CONFLICT, anyhow_err),
                "500" => ApiError::internal(anyhow_err),
                _ => ApiError::internal(anyhow_err),
            };
        }

        // Default: map to internal server error
        ApiError::internal(anyhow_err)
    }
}

// Map ceres-style coded errors like "[code:404] message" into ApiError with proper status.
pub(crate) fn map_ceres_error<D: std::fmt::Display>(err: D, ctx: &str) -> ApiError {
    let s = err.to_string();

    // Reuse the shared parse_error_code helper
    if let Some((code, msg)) = parse_error_code(&s) {
        let error_msg = anyhow::anyhow!(msg.to_string());
        return match code {
            "400" => ApiError::bad_request(error_msg),
            "404" => ApiError::not_found(error_msg),
            _ => ApiError::internal(error_msg),
        };
    }

    ApiError::internal(anyhow::anyhow!(format!("{}: {}", ctx, s)))
}
