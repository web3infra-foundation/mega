use api_model::common::CommonResult;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use common::errors::{ProtocolError, protocol_error_http_status, protocol_error_is_client_safe};

pub fn into_response(err: ProtocolError) -> Response {
    let status = StatusCode::from_u16(protocol_error_http_status(&err))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let err_msg = err.to_string();

    if status.is_client_error() {
        tracing::warn!(status = %status, "Protocol client error: {}", err_msg);
    } else {
        tracing::error!(status = %status, "Protocol error: {}", err_msg);
    }

    let response_msg = if protocol_error_is_client_safe(&err) {
        err_msg
    } else {
        "Something went wrong".to_owned()
    };

    (status, Json(CommonResult::<String>::failed(&response_msg))).into_response()
}

/// User-visible message for SSH stderr / channel data.
pub fn ssh_error_message(err: &ProtocolError) -> String {
    if protocol_error_is_client_safe(err) {
        format!("fatal: {err}\n")
    } else {
        "fatal: something went wrong\n".to_owned()
    }
}

/// Non-zero SSH exit status for protocol failures.
pub fn ssh_exit_status(err: &ProtocolError) -> u32 {
    match protocol_error_http_status(err) {
        404 => 1,
        401 | 403 => 126,
        400 | 413 => 2,
        _ => 1,
    }
}
