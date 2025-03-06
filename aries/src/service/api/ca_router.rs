use axum::{
    body::{to_bytes, Body},
    extract::{Query, State},
    http::{Request, Response, StatusCode, Uri},
    routing::get,
    Router,
};
use gemini::RelayGetParams;
use regex::Regex;

use crate::service::relay_server::AppState;

pub fn routers() -> Router<AppState> {
    Router::new().route("/{*path}", get(get_method_router).post(post_method_router))
}

async fn get_method_router(
    _state: State<AppState>,
    Query(_params): Query<RelayGetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/certificates/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        let name = match gemini::ca::server::get_cert_name_from_path(uri.path()) {
            Some(n) => n,
            None => {
                return Err((StatusCode::BAD_REQUEST, "Bad request".to_string()));
            }
        };
        return match gemini::ca::server::get_certificate(name).await {
            Ok(cert) => Ok(Response::builder().body(Body::from(cert)).unwrap()),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}

async fn post_method_router(
    _state: State<AppState>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/certificates/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        let name = match gemini::ca::server::get_cert_name_from_path(uri.path()) {
            Some(n) => n,
            None => return Err((StatusCode::BAD_REQUEST, "Bad request".to_string())),
        };
        let bytes = to_bytes(req.into_body(), usize::MAX).await.unwrap();
        let csr = String::from_utf8(bytes.to_vec()).unwrap();
        return match gemini::ca::server::issue_certificate(name, csr).await {
            Ok(cert) => Ok(Response::builder().body(Body::from(cert)).unwrap()),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}
