//! Per-request trace id for log correlation and client propagation.
//!
//! - **Inbound**: reads `X-Request-Id` or `X-Trace-Id` if present and valid; otherwise generates a UUID.
//! - **Storage**: [`TraceContext`] in request extensions for handlers and middleware.
//! - **Tracing**: [`http_request_span`] attaches `trace_id` to the `http.request` span (used by `TraceLayer`).
//! - **Outbound**: echoes the id as `X-Request-Id` on the response.
//!
//! Handlers can log the id explicitly: `tracing::info!(trace_id = %trace_id(req).unwrap_or("?"), "…")`.

use std::sync::Arc;

use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use http::header::{HeaderMap, HeaderValue};
use uuid::Uuid;

/// Request extension: stable trace id for this HTTP request.
#[derive(Clone, Debug)]
pub struct TraceContext {
    pub trace_id: Arc<str>,
}

const MAX_INBOUND_LEN: usize = 128;

/// Resolve trace id from headers or generate a new one.
pub fn resolve_trace_id(headers: &HeaderMap) -> Arc<str> {
    for name in ["x-request-id", "x-trace-id"] {
        if let Some(raw) = headers.get(name).and_then(|v| v.to_str().ok()) {
            let s = raw.trim();
            if is_valid_inbound_trace_token(s) {
                return Arc::from(s.to_string());
            }
        }
    }
    Arc::from(Uuid::new_v4().to_string())
}

fn is_valid_inbound_trace_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_INBOUND_LEN
        && s.bytes().all(|b| {
            matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b':' | b'/' )
        })
}

/// Returns the trace id if [`inject_trace_context`] ran for this request.
pub fn trace_id<B>(req: &Request<B>) -> Option<&str> {
    req.extensions()
        .get::<TraceContext>()
        .map(|c| c.trace_id.as_ref())
}

/// Outer middleware: set [`TraceContext`] and response `X-Request-Id` before inner layers (including `TraceLayer`).
pub async fn inject_trace_context(mut req: Request, next: Next) -> Response {
    let trace_id = resolve_trace_id(req.headers());
    req.extensions_mut().insert(TraceContext {
        trace_id: trace_id.clone(),
    });

    let mut res = next.run(req).await;
    if let Ok(val) = HeaderValue::from_str(trace_id.as_ref()) {
        res.headers_mut().insert("x-request-id", val);
    }
    res
}

/// Span for `tower_http::trace::TraceLayer` — must run **after** [`inject_trace_context`] on the same request.
pub fn http_request_span(request: &Request<Body>) -> tracing::Span {
    let trace_id = request
        .extensions()
        .get::<TraceContext>()
        .map(|c| c.trace_id.as_ref())
        .unwrap_or("unknown");
    tracing::info_span!(
        "http.request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
        trace_id = %trace_id,
    )
}
