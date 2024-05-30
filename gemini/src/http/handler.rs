use axum::{
    body::Body,
    http::{Response, StatusCode},
};

use crate::RelayGetParams;

pub async fn hello_gemini(_params: RelayGetParams) -> Result<Response<Body>, (StatusCode, String)> {
    Ok(Response::builder()
        .body(Body::from("hello gemini"))
        .unwrap())
}

#[cfg(test)]
mod tests {}
