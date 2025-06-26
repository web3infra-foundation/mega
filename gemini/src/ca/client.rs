use anyhow::Result;
use anyhow::{anyhow, Ok};
use quinn::rustls::pki_types::pem::PemObject;
use quinn::rustls::pki_types::CertificateDer;
use quinn::rustls::pki_types::PrivateKeyDer;
use rcgen::{CertificateParams, KeyPair};

use crate::p2p::client::P2PClient;

use super::{get_from_vault, save_to_vault};

static USER_KEY: &str = "user_key";

impl P2PClient {
    pub fn get_user_key(&self) -> String {
        match get_from_vault(&self.vault, USER_KEY.to_string()) {
            Some(key) => key,
            None => {
                let user_key = KeyPair::generate().unwrap();
                save_to_vault(&self.vault, USER_KEY.to_string(), user_key.serialize_pem());
                user_key.serialize_pem()
            }
        }
    }

    pub async fn get_user_cert_from_ca(
        &self,
        ca: impl AsRef<str>,
    ) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
        let name = self.get_peer_id();
        // Request to ca
        let url = format!("{}/api/v1/ca/certificates/{name}", ca.as_ref());
        let url = add_http_to_url(url);
        let response = self.http_client.get(url.clone()).send().await?;
        if response.status().is_success() {
            //cert exists
            let cert = response.text().await?;
            let cert = CertificateDer::from_pem_slice(cert.as_bytes())?;
            let key = self.get_user_key();
            let key = PrivateKeyDer::from_pem_slice(key.as_bytes())?;
            return Ok((cert, key));
        }

        let params = CertificateParams::new(vec![name])?;

        let key = self.get_user_key();
        let key = KeyPair::from_pem(&key)?;
        let user_csr = params.serialize_request(&key)?;
        //request a new cert
        let response = self
            .http_client
            .post(url)
            .body(user_csr.pem().unwrap())
            .send()
            .await
            .unwrap();

        if !response.status().is_success() {
            return Err(anyhow!("get user certificate from ca failed"));
        }

        let cert = CertificateDer::from_pem_slice(response.text().await.unwrap().as_bytes())?;
        let key = self.get_user_key();
        let key = PrivateKeyDer::from_pem_slice(key.as_bytes())?;
        Ok((cert, key))
    }

    pub async fn get_ca_cert_from_ca(
        &self,
        ca: impl AsRef<str>,
    ) -> Result<CertificateDer<'static>> {
        //request to ca
        let url = format!("{}/api/v1/ca/certificates/ca", ca.as_ref());
        let url = add_http_to_url(url);
        let response = self.http_client.get(url.clone()).send().await?;
        if response.status().is_success() {
            let cert = response.text().await?;
            let cert = CertificateDer::from_pem_slice(cert.as_bytes())?;
            return Ok(cert);
        }

        Err(anyhow!("get user certificate from ca failed"))
    }
}

fn add_http_to_url(url: String) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url;
    }

    let url = format!("http://{url}");
    url
}
