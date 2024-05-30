use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use common::config::RelayConfig;
use reqwest::Client;

use crate::RelayGetParams;

use super::{Agent, ZTMUserPermit};

pub async fn get_ztm_certificate(
    config: RelayConfig,
    params: RelayGetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    if params.name.is_none() {
        return Err((StatusCode::BAD_REQUEST, "not enough paras".to_string()));
    }
    let name = params.name.unwrap();
    let ca_address = config.ca;
    let hub_address = config.hub;

    //1. GET {ca}/api/certificates/ca -> ca certificate
    let url = format!("http://{ca_address}/api/certificates/ca");
    let request_result = reqwest::get(url).await;
    let ca_certificate = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, s));
        }
    };

    //2. POST {ca}/api/certificates/{username} -> user private key
    let url = format!("http://{ca_address}/api/certificates/{name}");
    let client = Client::new();
    let request_result = client.post(url).send().await;
    let user_key = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, s));
        }
    };

    //3. GET {ca}/api/certificates/{username} -> user certificate
    let url = format!("http://{ca_address}/api/certificates/{name}");
    let request_result = reqwest::get(url).await;
    let user_certificate = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, s));
        }
    };

    // Combine those into a json permit
    let agent = Agent {
        certificate: user_certificate.clone(),
        private_key: user_key.clone(),
    };

    let permit = ZTMUserPermit {
        ca: ca_certificate.clone(),
        agent,
        bootstraps: vec![hub_address],
    };

    let permit_json = serde_json::to_string(&permit).unwrap();
    tracing::info!("new permit [{name}]: {permit_json}");

    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(permit_json))
        .unwrap())
}

pub async fn handle_ztm_response(
    request_result: Result<reqwest::Response, reqwest::Error>,
) -> Result<String, String> {
    match request_result {
        Ok(res) => {
            if res.status().is_success() {
                Ok(res.text().await.unwrap())
            } else {
                Err(res.text().await.unwrap())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {}
