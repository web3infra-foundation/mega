use ::serde::{Deserialize, Serialize};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::util::handle_response;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CertAgent {
    pub name: String,
    pub certificate: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

#[derive(Debug, Clone)]
pub struct LocalZTMHub {
    pub hub_port: u16,
    pub ca: String,
    pub name: Vec<String>,
}

impl LocalZTMHub {
    pub fn start_ztm_hub(self) {
        tokio::spawn(async move {
            // neptune::start_hub(self.hub_port, self.name, &self.ca);
        });
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZTMUserPermit {
    pub ca: String,
    pub agent: CertAgent,
    pub bootstraps: Vec<String>,
}

impl ZTMUserPermit {
    pub fn to_json_map(&self) -> serde_json::Map<String, Value> {
        let value = serde_json::to_value(self.clone()).unwrap();
        serde_json::from_str(&value.to_string()).unwrap()
    }

    pub fn from_json_map(map: serde_json::Map<String, Value>) -> ZTMUserPermit {
        let permit: ZTMUserPermit = serde_json::from_value(Value::Object(map)).unwrap();
        permit
    }
}

#[async_trait]
pub trait ZTMCA {
    async fn create_ztm_certificate(&self, name: String) -> Result<ZTMUserPermit, String>;
    async fn delete_ztm_certificate(&self, name: String) -> Result<String, String>;
}

pub struct LocalHub {
    pub hub_host: String,
    pub hub_port: u16,
    pub ca_port: u16,
}

#[async_trait]
impl ZTMCA for LocalHub {
    async fn create_ztm_certificate(&self, name: String) -> Result<ZTMUserPermit, String> {
        let ca_port = self.ca_port;
        let ca_address = format!("http://localhost:{ca_port}");

        //1. GET {ca}/api/certificates/ca -> ca certificate
        let url = format!("{ca_address}/api/certificates/ca");
        let request_result = reqwest::get(url).await;
        let ca_certificate = match handle_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };

        //2. POST {ca}/api/certificates/{username} -> user private key
        let url = format!("{ca_address}/api/certificates/{name}");
        let client = Client::new();
        let request_result = client.post(url).send().await;
        let user_key = match handle_response(request_result).await {
            Ok(s) => s,
            Err(s) => {
                return Err(s);
            }
        };

        //3. GET {ca}/api/certificates/{username} -> user certificate
        let url = format!("{ca_address}/api/certificates/{name}");
        let request_result = reqwest::get(url).await;
        let user_certificate = match handle_response(request_result).await {
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

        let hub_address = format!("{}:{}", self.hub_host, self.hub_port);
        let permit = ZTMUserPermit {
            ca: ca_certificate.clone(),
            agent,
            bootstraps: vec![hub_address],
        };

        let permit_json = serde_json::to_string(&permit).unwrap();
        tracing::info!("new permit [{name}]: {permit_json}");

        Ok(permit)
    }

    async fn delete_ztm_certificate(&self, _name: String) -> Result<String, String> {
        return Err("not allowed".to_string());
    }
}
