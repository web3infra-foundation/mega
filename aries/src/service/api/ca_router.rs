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
    Router::new().route(
        "/{*path}",
        get(get_method_router)
            .post(post_method_router)
            .delete(delete_method_router),
    )
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
        let name = match gemini::ca::get_cert_name_from_path(uri.path()) {
            Some(n) => n,
            None => {
                return gemini::ca::response_error(
                    StatusCode::BAD_REQUEST.as_u16(),
                    "Bad request".to_string(),
                )
            }
        };
        return gemini::ca::get_certificate(name).await;
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
        let name = match gemini::ca::get_cert_name_from_path(uri.path()) {
            Some(n) => n,
            None => {
                return gemini::ca::response_error(
                    StatusCode::BAD_REQUEST.as_u16(),
                    "Bad request".to_string(),
                )
            }
        };
        return gemini::ca::issue_certificate(name).await;
    } else if Regex::new(r"/sign/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        let name = match gemini::ca::get_hub_name_from_path(uri.path()) {
            Some(n) => n,
            None => {
                return gemini::ca::response_error(
                    StatusCode::BAD_REQUEST.as_u16(),
                    "Bad request".to_string(),
                )
            }
        };
        let bytes = to_bytes(req.into_body(), usize::MAX).await.unwrap();
        let pubkey = String::from_utf8(bytes.to_vec()).unwrap();
        return gemini::ca::sign_certificate(name, pubkey).await;
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}

async fn delete_method_router(
    _state: State<AppState>,
    uri: Uri,
    _req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    if Regex::new(r"/certificates/[a-zA-Z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        return gemini::ca::delete_certificate(uri.path()).await;
    }
    Err((
        StatusCode::NOT_FOUND,
        String::from("Operation not supported\n"),
    ))
}
