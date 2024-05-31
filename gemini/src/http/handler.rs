use axum::{
    body::{to_bytes, Body},
    http::{Request, Response, StatusCode},
};
use common::config::ZTMConfig;

use crate::{
    ztm::{
        ZTMUserPermit, {connect_ztm_hub, create_ztm_certificate, create_ztm_service},
    },
    RelayGetParams,
};

pub async fn hello_gemini(_params: RelayGetParams) -> Result<Response<Body>, (StatusCode, String)> {
    Ok(Response::builder()
        .body(Body::from("hello gemini"))
        .unwrap())
}

pub async fn certificate(
    config: ZTMConfig,
    params: RelayGetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    if params.name.is_none() {
        return Err((StatusCode::BAD_REQUEST, "not enough paras".to_string()));
    }
    let name = params.name.unwrap();
    let permit = match create_ztm_certificate(config, name.clone()).await {
        Ok(p) => p,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
    };

    let permit_json = serde_json::to_string(&permit).unwrap();
    tracing::info!("new permit [{name}]: {permit_json}");

    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(permit_json))
        .unwrap())
}

pub async fn init(
    config: ZTMConfig,
    req: Request<Body>,
    relay_port: u16,
) -> Result<Response<Body>, (StatusCode, String)> {
    // transfer body to json
    let body_bytes = match to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    };

    let permit: ZTMUserPermit = match serde_json::from_slice(&body_bytes) {
        Ok(p) => p,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    };

    // 1. connect to ZTM hub (join a mesh)
    let mesh = match connect_ztm_hub(config.clone(), permit).await {
        Ok(m) => m,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
    };

    // 2. create a ZTM service
    let response_text =
        match create_ztm_service(config, mesh.agent.id, "relay".to_string(), relay_port).await {
            Ok(m) => m,
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
            }
        };

    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(response_text))
        .unwrap())
}

#[cfg(test)]
mod tests {}
