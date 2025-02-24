use anyhow::Result;
use anyhow::{anyhow, Ok};
use rcgen::{CertificateParams, KeyPair};
use reqwest::Client;
use vault::get_peerid;

use super::{get_from_vault, save_to_vault};

static USER_KEY: &str = "user_key";

pub async fn get_user_key() -> String {
    match get_from_vault(USER_KEY.to_string()).await {
        Some(key) => key,
        None => {
            let user_key = KeyPair::generate().unwrap();
            save_to_vault(USER_KEY.to_string(), user_key.serialize_pem()).await;
            user_key.serialize_pem()
        }
    }
}

pub async fn get_user_cert_from_ca(ca: String) -> Result<String> {
    let name = get_peerid().await;
    //request to ca
    let url = format!("{ca}/api/v1/ca/certificates/{name}");
    let url = add_http_to_url(url);
    let client = Client::new();
    let response = client.get(url.clone()).send().await.unwrap();
    if response.status().is_success() {
        //cert exists
        return Ok(response.text().await.unwrap());
    }

    let params = CertificateParams::new(vec![name]).unwrap();

    let key = get_user_key().await;
    let key = KeyPair::from_pem(&key).unwrap();
    let user_csr = params.serialize_request(&key).unwrap();

    //request a new cert
    let response = client
        .post(url)
        .body(user_csr.pem().unwrap())
        .send()
        .await
        .unwrap();
    if response.status().is_success() {
        return Ok(response.text().await.unwrap());
    }

    Err(anyhow!("get user certificate from ca failed"))
}

pub async fn get_ca_cert_from_ca(ca: String) -> Result<String> {
    //request to ca
    let url = format!("{ca}/api/v1/ca/certificates/ca");
    let url = add_http_to_url(url);
    let client = Client::new();
    let response = client.get(url.clone()).send().await?;
    if response.status().is_success() {
        return Ok(response.text().await?);
    }

    Err(anyhow!("get user certificate from ca failed"))
}

fn add_http_to_url(url: String) -> String {
    if url.starts_with("http://") {
        return url;
    }

    let url = format!("http://{url}");
    url
}
