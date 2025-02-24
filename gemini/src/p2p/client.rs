use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Ok;
use anyhow::Result;
use quinn::crypto::rustls::QuicClientConfig;
use quinn::rustls::pki_types::pem::PemObject;
use quinn::rustls::pki_types::CertificateDer;
use quinn::rustls::pki_types::PrivateKeyDer;
use quinn::{rustls, ClientConfig, Endpoint};
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;
use vault::get_peerid;

use crate::ca;
use crate::p2p::relay::{ReceiveData, SenderData};
use crate::p2p::Action;

use super::ALPN_QUIC_HTTP;

pub async fn run(bootstrap_node: String) -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let endpoint = get_client_endpoint(bootstrap_node.clone()).await?;

    let server_addr: SocketAddr = bootstrap_node.parse()?;
    let connection = endpoint
        .connect(server_addr, "localhost")?
        .await
        .map_err(|e| anyhow!("failed to connect: {}", e))?;

    let remote_address = connection.remote_address();
    let stable_id = connection.stable_id();
    info!("Established connection: {remote_address:#?},{stable_id:#?}");

    let connection = Arc::new(connection);
    let connection_clone = connection.clone();

    let (tx, mut rx) = mpsc::channel(8);

    let peer_id = vault::get_peerid().await;

    tokio::spawn(async move {
        loop {
            let (mut quic_send, _) = connection_clone.open_bi().await.unwrap();
            let ping = ReceiveData {
                from: peer_id.clone(),
                data: vec![],
                func: "".to_string(),
                action: Action::Ping,
                to: "".to_string(),
                req_id: Uuid::new_v4().into(),
            };
            let json = serde_json::to_string(&ping).unwrap();
            quic_send.write_all(json.as_ref()).await.unwrap();
            quic_send.finish().unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    let connection_clone = connection.clone();
    tokio::spawn(async move {
        loop {
            let (_, mut quic_recv) = connection_clone.accept_bi().await.unwrap();
            let buffer = quic_recv.read_to_end(1024 * 1024).await.unwrap();
            info!("QUIC Received:\n{}", String::from_utf8_lossy(&buffer));
            if tx.send(buffer).await.is_err() {
                info!("Receiver closed");
                return;
            }
        }
    });

    while let Some(message) = rx.recv().await {
        //TODO with the message
        info!(
            "Channel Received message: {}",
            String::from_utf8_lossy(&message)
        );
    }

    Ok(())
}

pub async fn get_client_endpoint(bootstrap_node: String) -> anyhow::Result<Endpoint> {
    let (user_cert, user_key) = get_user_cert_from_ca(bootstrap_node.clone()).await?;
    let ca_cert = get_ca_cert_from_ca(bootstrap_node.clone()).await?;

    let mut roots = rustls::RootCertStore::empty();

    roots.add(ca_cert).unwrap();

    let mut client_crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert([user_cert].to_vec(), user_key)
        .unwrap();
    client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    client_crypto.enable_early_data = true;
    let client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));
    let mut endpoint = Endpoint::client(SocketAddr::from_str("127.0.0.1:0").unwrap())?;
    endpoint.set_default_client_config(client_config);

    Ok(endpoint)
}

pub async fn send(
    to_peer_id: String,
    func: String,
    data: Vec<u8>,
    bootstrap_node: String,
) -> Result<Vec<u8>> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let endpoint = get_client_endpoint(bootstrap_node.clone()).await?;

    let server_addr: SocketAddr = bootstrap_node.parse()?;
    let connection = endpoint
        .connect(server_addr, "localhost")?
        .await
        .map_err(|e| anyhow!("failed to connect: {}", e))?;

    let remote_address = connection.remote_address();
    let stable_id = connection.stable_id();
    info!("established connection: {remote_address:#?},{stable_id:#?}");
    let connection = Arc::new(connection);

    let connection_clone = connection.clone();
    let local_peer_id = get_peerid().await;
    tokio::spawn(async move {
        let (mut sender, _) = connection_clone.open_bi().await.unwrap();
        let send = ReceiveData {
            from: local_peer_id.clone(),
            data: data.clone(),
            func: func.to_string(),
            action: Action::Send,
            to: to_peer_id.to_string(),
            req_id: Uuid::new_v4().into(),
        };
        let json = serde_json::to_string(&send).unwrap();
        sender.write_all(json.as_bytes()).await.unwrap();
        sender.finish().unwrap();
    });

    let connection_clone = connection.clone();

    tokio::spawn(async move {
        let (_, mut quic_recv) = connection_clone.accept_bi().await.unwrap();
        let buffer = quic_recv.read_to_end(1024 * 1024).await.unwrap();
        info!("QUIC Received:\n{}", String::from_utf8_lossy(&buffer));
        if tx.send(buffer).is_err() {
            info!("Receiver closed");
        }
    });
    let message = rx.await?;
    info!(
        "Channel Received message: {}",
        String::from_utf8_lossy(&message)
    );
    let data: SenderData = serde_json::from_slice(&message)?;
    Ok(data.data)
}

pub async fn get_user_cert_from_ca(
    ca: String,
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let cert = ca::client::get_user_cert_from_ca(ca).await?;
    let cert = CertificateDer::from_pem_slice(cert.as_bytes())?;
    let key = ca::client::get_user_key().await;
    let key = PrivateKeyDer::from_pem_slice(key.as_bytes())?;
    Ok((cert, key))
}

pub async fn get_ca_cert_from_ca(ca: String) -> Result<CertificateDer<'static>> {
    let cert = ca::client::get_ca_cert_from_ca(ca).await?;
    let cert = CertificateDer::from_pem_slice(cert.as_bytes())?;
    Ok(cert)
}
