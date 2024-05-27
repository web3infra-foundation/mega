use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use common::model::GetParams;

pub async fn hello_gemini(_params: GetParams) -> Result<Response<Body>, (StatusCode, String)> {
    return Ok(Response::builder()
        .body(Body::from("hello gemini"))
        .unwrap());
}

#[cfg(test)]
mod tests {}
