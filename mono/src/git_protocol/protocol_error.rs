use api_model::common::CommonResult;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use common::errors::ProtocolError;

pub fn into_response(err: ProtocolError) -> Response {
    let (status, message) = match err {
        ProtocolError::Deny(err) => (StatusCode::UNAUTHORIZED, err),
        ProtocolError::TooLarge(err) => (StatusCode::PAYLOAD_TOO_LARGE, err),
        ProtocolError::NotFound(err) => (StatusCode::NOT_FOUND, err),
        ProtocolError::InvalidInput(err) => (StatusCode::BAD_REQUEST, err),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong".to_owned(),
        ),
    };

    (status, Json(CommonResult::<String>::failed(&message))).into_response()
}
