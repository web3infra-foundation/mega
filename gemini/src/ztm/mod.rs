use axum::async_trait;
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
pub struct ZTMEndPoint {
    pub id: String,
    pub name: String,
    pub username: String,
    pub online: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ZTMServiceReq {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ZTMPortReq {
    pub target: ZTMPortService,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ZTMPortService {
    pub service: String,
}

const MESH_NAME: &str = "relay_mesh";

#[async_trait]
pub trait ZTM {
    async fn create_ztm_certificate(&self, name: String) -> Result<ZTMUserPermit, String>;
    // fn create_ztm_certificate(&self, name: String) -> Result<ZTMUserPermit, String>;
    async fn delete_ztm_certificate(&self, name: String) -> Result<String, String>;
    async fn connect_ztm_hub(&self, permit: ZTMUserPermit) -> Result<ZTMMesh, String>;
    async fn create_ztm_service(
        &self,
        ep_id: String,
        service_name: String,
        port: u16,
    ) -> Result<String, String>;
    async fn create_ztm_port(
        &self,
        ep_id: String,
        service_name: String,
        port: u16,
    ) -> Result<String, String>;

    async fn search_ztm_endpoint(&self, name: String) -> Result<ZTMEndPoint, String>;
}

pub struct RemoteZTM {
    pub config: ZTMConfig,
}

#[async_trait]
impl ZTM for RemoteZTM {
    async fn create_ztm_certificate(&self, name: String) -> Result<ZTMUserPermit, String> {
        let ca_address = self.config.ca.clone();
        let hub_address = self.config.hub.clone();

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

    async fn delete_ztm_certificate(&self, name: String) -> Result<String, String> {
        let ca_address = self.config.ca.clone();

        //1. DELETE /api/certificates/${username}
        let url = format!("{ca_address}/api/certificates/{name}");
        let client = Client::new();
        let request_result = client.delete(url).send().await;
        let s: String = match handle_ztm_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };
        Ok(s)
    }

    async fn connect_ztm_hub(&self, permit: ZTMUserPermit) -> Result<ZTMMesh, String> {
        // POST {agent}/api/meshes/${meshName}
        let permit_string = serde_json::to_string(&permit).unwrap();
        let agent_address = self.config.agent.clone();
        let url = format!("{agent_address}/api/meshes/{MESH_NAME}");
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

    async fn create_ztm_service(
        &self,
        ep_id: String,
        service_name: String,
        port: u16,
    ) -> Result<String, String> {
        //  create a ZTM service
        //  POST {agent}/api/meshes/${mesh.name}/endpoints/${ep.id}/services/${svcName}
        let agent_address = self.config.agent.clone();
        let url = format!(
            "{agent_address}/api/meshes/{MESH_NAME}/endpoints/{ep_id}/services/tcp/{service_name}"
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

    async fn create_ztm_port(
        &self,
        ep_id: String,
        service_name: String,
        port: u16,
    ) -> Result<String, String> {
        //POST {agent}/api/meshes/${mesh.name}/endpoints/${ep.id}/ports/127.0.0.1/tcp/{port}
        // request body {"service":service_name}
        let agent_address = self.config.agent.clone();
        let url = format!(
            "{agent_address}/api/meshes/{MESH_NAME}/endpoints/{ep_id}/ports/127.0.0.1/tcp/{port}"
        );
        let client = Client::new();
        let req = ZTMPortReq {
            target: ZTMPortService {
                service: service_name,
            },
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

    async fn search_ztm_endpoint(&self, name: String) -> Result<ZTMEndPoint, String> {
        // GET {agent}/api/meshes/{mesh}/endpoints
        let agent_address = self.config.agent.clone();
        let url = format!("{agent_address}/api/meshes/{MESH_NAME}/endpoints");
        let request_result = reqwest::get(url).await;
        let response_text = match handle_ztm_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };

        let endpoint_list: Vec<ZTMEndPoint> = match serde_json::from_slice(response_text.as_bytes())
        {
            Ok(p) => p,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        for endpoint in endpoint_list {
            if endpoint.name == name {
                return Ok(endpoint);
            }
        }
        return Err("endpoint not found".to_string());
    }
}

pub async fn run_ztm_client(bootstrap_node: String, config: ZTMConfig, peer_id: String) {
    let name = peer_id;
    let ztm: RemoteZTM = RemoteZTM { config };

    // 1. to get permit json from bootstrap_node
    // GET {bootstrap_node}/api/v1/certificate?name={name}
    let url = format!("{bootstrap_node}/api/v1/certificate?name={name}");
    let request_result = reqwest::get(url).await;
    let response_text = match handle_ztm_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            tracing::error!("GET {bootstrap_node}/api/v1/certificate?name={name} failed,{s}");
            return;
        }
    };
    let permit: ZTMUserPermit = match serde_json::from_slice(response_text.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("{}", e);
            return;
        }
    };

    // 2. join ztm mesh
    let mesh = match ztm.connect_ztm_hub(permit).await {
        Ok(m) => m,
        Err(s) => {
            tracing::error!(s);
            return;
        }
    };
    tracing::info!("connect to ztm hub successfully");

    // // 3. find ztm relay service
    // let relay_endpoint = match ztm.search_ztm_endpoint("relay".to_string()).await {
    //     Ok(endpoint) => endpoint,
    //     Err(s) => {
    //         tracing::error!("find relay ztm endpoint failed, {s}");
    //         return;
    //     }
    // };
    // let endpoint_id = relay_endpoint.id;
    // tracing::info!("find relay ztm endpoint successfully, id = {endpoint_id}");

    // 4. create a ztm port
    let ztm_port = 8002;
    match ztm
        .create_ztm_port(mesh.agent.id, "relay".to_string(), ztm_port)
        .await
    {
        Ok(_) => (),
        Err(s) => {
            tracing::error!("create a ztm port failed, {s}");
            return;
        }
    }
    tracing::info!("create a ztm port successfully, port:{ztm_port}");
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
