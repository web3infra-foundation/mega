use axum::{body::Body, response::Response};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ErrorResult {
    status: u16,
    #[serde(rename = "message")]
    msg: String,
}

impl ErrorResult {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    pub fn to_json_string_new(status: u16, msg: String) -> String {
        let e = ErrorResult { status, msg };
        e.to_json_string()
    }
}

pub async fn get_certificate(name: String) -> Result<Response, (StatusCode, String)> {
    if name == "ca" {
        return Ok(Response::builder()
            .body(Body::from(vault::pki::get_root_cert().await))
            .unwrap());
    }
    let cert_option = get_from_vault(name).await;
    match cert_option {
        Some(cert) => Ok(Response::builder().body(Body::from(cert)).unwrap()),
        None => response_error(
            StatusCode::NOT_FOUND.as_u16(),
            "Username not found".to_string(),
        ),
    }
}

pub async fn issue_certificate(name: String) -> Result<Response, (StatusCode, String)> {
    if is_reserved_name(name.clone()) {
        return response_error(
            StatusCode::FORBIDDEN.as_u16(),
            "Reserved username".to_string(),
        );
    }
    // let cert_option = get_from_vault(name.clone());
    // if cert_option.is_some() {
    //     return response_error(
    //         StatusCode::CONFLICT.as_u16(),
    //         "Username already exists".to_string(),
    //     );
    // }
    let (cert_pem, private_key) = vault::pki::issue_cert(json!({
        "ttl": "10d",
        "common_name": name,
    })).await;
    //save cert to vault
    save_to_vault(name, cert_pem).await;
    Ok(Response::builder().body(Body::from(private_key)).unwrap())
}

pub async fn sign_certificate(
    name: String,
    pubkey: String,
) -> Result<Response, (StatusCode, String)> {
    tracing::info!("sign_certificate,name:{name},pubkey:{pubkey}");
    if is_reserved_name(name.clone()) {
        return response_error(
            StatusCode::FORBIDDEN.as_u16(),
            "Reserved username".to_string(),
        );
    }
    let cert_option = get_from_vault(name.clone()).await;
    if cert_option.is_some() {
        return response_error(
            StatusCode::CONFLICT.as_u16(),
            "Username already exists".to_string(),
        );
    }
    let (cert_pem, private_key) = vault::pki::issue_cert(json!({
        "ttl": "10d",
        "common_name": name,
    })).await;
    //save cert to vault
    save_to_vault(name, cert_pem).await;
    Ok(Response::builder().body(Body::from(private_key)).unwrap())
}

pub async fn delete_certificate(path: &str) -> Result<Response, (StatusCode, String)> {
    let name = match get_cert_name_from_path(path) {
        Some(n) => n,
        None => return response_error(StatusCode::BAD_REQUEST.as_u16(), "Bad request".to_string()),
    };
    if is_reserved_name(name.clone()) {
        return response_error(
            StatusCode::FORBIDDEN.as_u16(),
            "Reserved username".to_string(),
        );
    }
    delete_to_vault(name);
    Ok(Response::builder()
        .status(204)
        .body(Body::from(""))
        .unwrap())
}

pub fn get_cert_name_from_path(path: &str) -> Option<String> {
    let v: Vec<&str> = path.split('/').collect();
    v.get(3).map(|s| s.to_string())
}

pub fn get_hub_name_from_path(path: &str) -> Option<String> {
    let v: Vec<&str> = path.split('/').collect();
    v.get(4).map(|s| s.to_string())
}

fn is_reserved_name(name: String) -> bool {
    if name == "ca" {
        return true;
    }
    is_hub_name(name)
}

fn is_hub_name(_name: String) -> bool {
    // if name == "hub" || name.starts_with("hub/") {
    //     return true;
    // }
    // false
    false
}

async fn save_to_vault(key: String, value: String) {
    let key_f = format!("ca_{key}");
    let kv_data = json!({
        key_f.clone(): value,
    })
    .as_object()
    .unwrap()
    .clone();
    vault::vault::write_secret(key_f.as_str(), Some(kv_data.clone())).await.unwrap();
}

async fn get_from_vault(key: String) -> Option<String> {
    let key_f = format!("ca_{key}");
    let secret = match vault::vault::read_secret(key_f.as_str()).await.unwrap() {
        Some(res) => res.data,
        None => return None,
    };

    match secret {
        Some(m) => {
            let s = m.get(key_f.as_str()).unwrap().as_str().unwrap().to_string();
            let s = s.trim_matches(char::is_control).to_string();
            Some(s)
        }
        None => None,
    }
}

fn delete_to_vault(_key: String) {
    // let key_f = format!("ca_{key}");
    // vault::vault::write_secret(key_f.as_str(), Some(Map).unwrap());
}

pub fn response_error(status: u16, message: String) -> Result<Response, (StatusCode, String)> {
    Ok({
        let error_result = ErrorResult::to_json_string_new(status, message);
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(error_result))
            .unwrap()
    })
}
