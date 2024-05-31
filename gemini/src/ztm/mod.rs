use common::config::ZTMConfig;
use reqwest::{header::CONTENT_TYPE, Client};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ZTMUserPermit {
    pub ca: String,
    pub agent: CertAgent,
    pub bootstraps: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CertAgent {
    pub name: String,
    pub certificate: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ZTMMesh {
    pub name: String,
    pub ca: String,
    pub agent: Agent,
    pub bootstraps: Vec<String>,
    pub connected: bool,
    pub errors: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub username: String,
    pub certificate: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ZTMServiceReq {
    pub host: String,
    pub port: u16,
}

/// create ztm certificate
/// ztm.ca and ztm.hub in config.toml are required
/// # Arguments
/// * `config` - ZTMConfig
/// * `name` - String
///
/// # Returns
/// * ZTMUserPermit
///
/// ZTMUserPermit include ca_certificate,user_certificate,user_key
/// ```
pub async fn create_ztm_certificate(
    config: ZTMConfig,
    name: String,
) -> Result<ZTMUserPermit, String> {
    let ca_address = config.ca;
    let hub_address = config.hub;

    //1. GET {ca}/api/certificates/ca -> ca certificate
    let url = format!("{ca_address}/api/certificates/ca");
    let request_result = reqwest::get(url).await;
    let ca_certificate = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err(s);
        }
    };

    //2. POST {ca}/api/certificates/{username} -> user private key
    let url = format!("{ca_address}/api/certificates/{name}");
    let client = Client::new();
    let request_result = client.post(url).send().await;
    let user_key = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err(s);
        }
    };

    //3. GET {ca}/api/certificates/{username} -> user certificate
    let url = format!("{ca_address}/api/certificates/{name}");
    let request_result = reqwest::get(url).await;
    let user_certificate = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err(s);
        }
    };

    // Combine those into a json permit
    let agent = CertAgent {
        name: name.clone(),
        certificate: user_certificate.clone(),
        private_key: user_key.clone(),
    };

    let hub_address = hub_address.replace("http://", "");
    let permit = ZTMUserPermit {
        ca: ca_certificate.clone(),
        agent,
        bootstraps: vec![hub_address],
    };

    let permit_json = serde_json::to_string(&permit).unwrap();
    tracing::info!("new permit [{name}]: {permit_json}");

    Ok(permit)
}

/// connect to hub (join a mesh)
/// ztm.agent in config.toml is required
/// # Arguments
/// * `config` - ZTMConfig
/// * `permit` - ZTMUserPermit
///
/// # Returns
/// * ZTMMesh
///
/// ```
pub async fn connect_ztm_hub(config: ZTMConfig, permit: ZTMUserPermit) -> Result<ZTMMesh, String> {
    // POST {agent}/api/meshes/${meshName}
    let permit_string = serde_json::to_string(&permit).unwrap();
    let agent_address = config.agent;
    let url = format!("{agent_address}/api/meshes/relay_mesh");
    let client = Client::new();
    let request_result = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(permit_string)
        .send()
        .await;
    let response_text = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err(s);
        }
    };

    let mesh: ZTMMesh = match serde_json::from_slice(response_text.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            return Err(e.to_string());
        }
    };
    Ok(mesh)
}

/// create ZTM Service
/// ztm.agent in config.toml is required
/// # Arguments
/// * `config` - ZTMConfig
/// * `ep_id` - String
/// * `service_name` - String
/// * `port` - u16
///
/// # Returns
/// * String
///
/// ```
pub async fn create_ztm_service(
    config: ZTMConfig,
    ep_id: String,
    service_name: String,
    port: u16,
) -> Result<String, String> {
    //  create a ZTM service
    //  POST {agent}/api/meshes/${mesh.name}/endpoints/${ep.id}/services/${svcName}
    let agent_address = config.agent;
    let url = format!(
        "{agent_address}/api/meshes/relay_mesh/endpoints/{ep_id}/services/tcp/{service_name}"
    );
    let client = Client::new();
    let req = ZTMServiceReq {
        host: "127.0.0.1".to_string(),
        port,
    };
    let req_string = serde_json::to_string(&req).unwrap();
    let request_result = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(req_string)
        .send()
        .await;
    let response_text = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            return Err(s);
        }
    };
    Ok(response_text)
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
