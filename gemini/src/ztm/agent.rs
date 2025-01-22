use std::{
    collections::HashMap,
    fs::{self, remove_dir_all},
    path::Path,
    time::Duration,
};

use async_trait::async_trait;
use common::config::Config;
use reqwest::{header::CONTENT_TYPE, Client};
use serde::{Deserialize, Serialize};

use crate::{
    ztm::{ZTM_APP_NAME, ZTM_APP_PROVIDER},
    RepoInfo,
};

use super::{handle_response, hub::ZTMUserPermit, MESH_NAME};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMMesh {
    pub name: String,
    pub ca: String,
    pub agent: Agent,
    pub bootstraps: Vec<String>,
    pub connected: bool,
    pub errors: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub username: String,
    pub certificate: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMEndPoint {
    pub id: String,
    pub username: String,
    pub name: String,
    pub online: bool,
    #[serde(rename = "isLocal")]
    pub is_local: bool,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMServiceReq {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMPortReq {
    pub target: ZTMPortService,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMPortService {
    pub service: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMAppInbound {
    pub protocol: String,
    pub name: String,
    pub listens: Vec<ZTMAppInboundListen>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMAppInboundListen {
    pub ip: String,
    pub port: u16,
}

#[async_trait]
pub trait ZTMAgent {
    async fn connect_ztm_hub(&self, permit: ZTMUserPermit) -> Result<ZTMMesh, String>;
    async fn get_ztm_endpoints(&self) -> Result<Vec<ZTMEndPoint>, String>;
    async fn get_ztm_local_endpoint(&self) -> Result<ZTMEndPoint, String>;
    async fn get_ztm_remote_endpoint(&self, peer_id: String) -> Result<ZTMEndPoint, String>;
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

    async fn start_ztm_app(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
    ) -> Result<String, String>;

    async fn create_ztm_app_tunnel_inbound(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
        bound_name: String,
        port: u16,
    ) -> Result<String, String>;

    async fn get_ztm_app_tunnel_inbound_port(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
        bound_name: String,
    ) -> Option<u16>;

    async fn create_ztm_app_tunnel_outbound(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
        bound_name: String,
        port: u16,
    ) -> Result<String, String>;
}

#[derive(Debug, Clone)]
pub struct LocalZTMAgent {
    pub agent_port: u16,
}

impl LocalZTMAgent {
    pub fn start_ztm_agent(self) {
        tokio::spawn(async move {
            // neptune::start_agent("ztm_agent_db", self.agent_port);
            let db_path = "ztm_agent_db";
            let path = Path::new(db_path);
            if fs::metadata(path).is_ok() {
                remove_dir_all(path).unwrap();
            }

            // neptune::start_agent(db_path, self.agent_port);
        });
    }
}

#[async_trait]
impl ZTMAgent for LocalZTMAgent {
    async fn connect_ztm_hub(&self, permit: ZTMUserPermit) -> Result<ZTMMesh, String> {
        // POST {agent}/api/meshes/${meshName}
        let permit_string = serde_json::to_string(&permit).unwrap();
        // tracing::info!("permit_string:{permit_string}");
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
        let url = format!("{agent_address}/api/meshes/{MESH_NAME}");
        let client = Client::new();
        let request_result = client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(permit_string)
            .send()
            .await;
        let response_text = match handle_response(request_result).await {
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

    async fn get_ztm_endpoints(&self) -> Result<Vec<ZTMEndPoint>, String> {
        //GET localhost:7777/api/meshes/{MESH_NAME}/endpoints
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
        let url = format!("{agent_address}/api/meshes/{MESH_NAME}/endpoints");
        let request_result = reqwest::get(url).await;
        let eps: String = match handle_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };
        let ep_list: Vec<ZTMEndPoint> = match serde_json::from_slice(eps.as_bytes()) {
            Ok(p) => p,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        Ok(ep_list)
    }

    async fn get_ztm_local_endpoint(&self) -> Result<ZTMEndPoint, String> {
        match self.get_ztm_endpoints().await {
            Ok(ep_list) => {
                for ele in ep_list {
                    if ele.is_local {
                        return Ok(ele);
                    }
                }
                return Err("Can not find local ztm endpoint".to_string());
            }
            Err(e) => Err(e),
        }
    }

    async fn get_ztm_remote_endpoint(&self, peer_id: String) -> Result<ZTMEndPoint, String> {
        match self.get_ztm_endpoints().await {
            Ok(ep_list) => {
                for ele in ep_list {
                    if ele.online && ele.name == peer_id {
                        return Ok(ele);
                    }
                }
                return Err(format!("Ztm agent({peer_id}) is not online."));
            }
            Err(e) => Err(e),
        }
    }

    async fn create_ztm_service(
        &self,
        ep_id: String,
        service_name: String,
        port: u16,
    ) -> Result<String, String> {
        //  create a ZTM service
        //  POST {agent}/api/meshes/${mesh.name}/endpoints/${ep.id}/services/${svcName}
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
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
        let response_text = match handle_response(request_result).await {
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
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
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
        let response_text = match handle_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };
        Ok(response_text)
    }

    async fn start_ztm_app(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
    ) -> Result<String, String> {
        //POST /api/meshes/${mesh.name}/endpoints/${ep.id}/apps/${provider}/${name}
        // request body { isRunning: true }
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
        let url = format!(
            "{agent_address}/api/meshes/{MESH_NAME}/endpoints/{ep_id}/apps/{provider}/{app_name}"
        );
        let client = Client::new();
        let req = r#"{"isRunning":true}"#;
        let request_result = client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(req)
            .send()
            .await;
        let response_text = match handle_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };
        Ok(response_text)
    }

    async fn create_ztm_app_tunnel_inbound(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
        bound_name: String,
        port: u16,
    ) -> Result<String, String> {
        //POST /api/meshes/{mesh.name}/apps/${provider}/${name}/api/endpoints/{ep}/inbound/{proto}/{name}
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
        let url = format!(
            "{agent_address}/api/meshes/{MESH_NAME}/apps/{provider}/{app_name}/api/endpoints/{ep_id}/inbound/tcp/{bound_name}"
        );
        tracing::info!("create_ztm_app_tunnel_inbound url: {}", url);
        let client = Client::new();
        let req = format!(r#"{{"listens": [{{"ip":"127.0.0.1","port":{port}}}]}}"#);
        // let req = r#"{"listens":[{"ip":"127.0.0.1","port":8081}]}"#;
        let request_result = client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(req)
            .send()
            .await;
        let response_text = match handle_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };
        Ok(response_text)
    }

    async fn get_ztm_app_tunnel_inbound_port(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
        bound_name: String,
    ) -> Option<u16> {
        //GET /api/meshes/{mesh.name}/apps/${provider}/${name}/api/endpoints/{ep}/inbound/{proto}/{name}
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
        let url = format!(
            "{agent_address}/api/meshes/{MESH_NAME}/apps/{provider}/{app_name}/api/endpoints/{ep_id}/inbound"
        );
        tracing::debug!("get_ztm_app_tunnel_inbound url: {}", url);
        let client = Client::new();
        let request_result = client.get(url).send().await;
        let response_text = match handle_response(request_result).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("get_ztm_app_tunnel_inbound error: {}", e);
                return None;
            }
        };
        let ztm_app_inbounds: Vec<ZTMAppInbound> =
            match serde_json::from_str(response_text.as_str()) {
                Ok(inbounds) => inbounds,
                Err(e) => {
                    tracing::error!("get_ztm_app_tunnel_inbound error: {}", e);
                    return None;
                }
            };
        let ztm_app_inbound = ztm_app_inbounds.iter().find(|x| x.name == bound_name)?;
        let listen = ztm_app_inbound.listens.first()?;
        Some(listen.port)
    }

    async fn create_ztm_app_tunnel_outbound(
        &self,
        ep_id: String,
        provider: String,
        app_name: String,
        bound_name: String,
        port: u16,
    ) -> Result<String, String> {
        //POST /api/meshes/{mesh.name}/apps/${provider}/${name}/api/endpoints/{ep}/outbound/{proto}/{name}
        let agent_port = self.agent_port;
        let agent_address = format!("http://127.0.0.1:{agent_port}");
        let url = format!(
            "{agent_address}/api/meshes/{MESH_NAME}/apps/{provider}/{app_name}/api/endpoints/{ep_id}/outbound/tcp/{bound_name}"
        );
        let client = Client::new();
        let req = format!(r#"{{"targets": [{{"host":"127.0.0.1","port":{port}}}]}}"#);
        // let req = r#"{"targets":[{"host":"127.0.0.1","port":{}}]}"#;
        let request_result = client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(req)
            .send()
            .await;
        let response_text = match handle_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };
        Ok(response_text)
    }
}

pub async fn run_ztm_client(
    bootstrap_node: String,
    _config: Config,
    peer_id: String,
    agent: LocalZTMAgent,
    http_port: u16,
) {
    //TODO ztm permit control?
    // let name = peer_id.clone();
    let url = format!("{bootstrap_node}/api/v1/certificate?name=root");
    let request_result = reqwest::get(url).await;
    let response_text = match handle_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            tracing::error!("GET {bootstrap_node}/api/v1/certificate?name=root failed,{s}");
            return;
        }
    };
    let mut permit: ZTMUserPermit = match serde_json::from_slice(response_text.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("{}", e);
            return;
        }
    };
    permit.agent.name = peer_id.clone();
    // 2. join ztm mesh
    let mesh = match agent.connect_ztm_hub(permit.clone()).await {
        Ok(m) => m,
        Err(s) => {
            tracing::error!("join ztm mesh failed");
            tracing::error!(s);
            return;
        }
    };
    tracing::info!("connect to ztm hub successfully");

    // 3. start tunnel app
    // /api/meshes/${mesh.name}/endpoints/${ep.id}/apps/${provider}/${name}
    match agent
        .start_ztm_app(
            mesh.clone().agent.id,
            ZTM_APP_PROVIDER.to_string(),
            ZTM_APP_NAME.to_string(),
        )
        .await
    {
        Ok(_) => (),
        Err(s) => {
            tracing::error!("start tunnel app failed, {s}");
            return;
        }
    }
    tracing::info!("start tunnel app successfully");

    // ping relay
    let peer_id_clone = peer_id.clone();
    let bootstrap_node_clone = bootstrap_node.clone();
    let mut interval = tokio::time::interval(Duration::from_secs(45));
    let url = format!("{bootstrap_node_clone}/api/v1/ping");
    loop {
        ping(
            url.clone(),
            peer_id_clone.clone(),
            permit.bootstraps.first().unwrap().to_string(),
            http_port,
        )
        .await;
        interval.tick().await;
    }
}

pub async fn ping(url: String, peer_id: String, hub: String, service_port: u16) {
    let mut params = HashMap::new();
    params.insert("peer_id", peer_id.clone());
    params.insert("hub", hub);
    params.insert("agent_name", peer_id.clone());
    params.insert("service_name", peer_id.clone());
    params.insert("service_port", service_port.to_string());
    let client = reqwest::Client::new();
    let response = client.get(url.clone()).query(&params).send().await;
    let response_text = match handle_response(response).await {
        Ok(s) => s,
        Err(s) => {
            tracing::error!("GET {url} failed,{s}");
            return;
        }
    };
    tracing::info!("Get {url}, response: {response_text}");
}

pub async fn share_repo(url: String, repo_info: RepoInfo) {
    let client = Client::new();
    tracing::info!("share_repo ,{:?}", repo_info);
    let req_string = serde_json::to_string(&repo_info).unwrap();
    let response = client
        .post(url.clone())
        .header(CONTENT_TYPE, "application/json")
        .body(req_string)
        .send()
        .await;
    let response_text = match handle_response(response).await {
        Ok(s) => s,
        Err(s) => {
            tracing::error!("POST {} failed,{}", url.clone(), s.clone());
            return;
        }
    };
    tracing::info!("POST {}, response: {}", url.clone(), response_text);
}
