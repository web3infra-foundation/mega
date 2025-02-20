use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::{bail, Context, Result};
use quinn::crypto::rustls::QuicClientConfig;
use quinn::{rustls, ClientConfig, Connection, Endpoint};
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;
use vault::get_peerid;

use crate::p2p::relay::{ReceiveData, SenderData};
use crate::p2p::Action;

use super::{get_certificate, ALPN_QUIC_HTTP};

pub async fn run(bootstrap_node: String) -> Result<()> {
    info!("Start");
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let connection = get_client_connection(bootstrap_node).await?;

    let remote_address = connection.remote_address();
    let stable_id = connection.stable_id();
    info!("Established connection: {remote_address:#?},{stable_id:#?}");

    let connection = Arc::new(connection);
    let connection_clone = connection.clone();

    let (tx, mut rx) = mpsc::channel(8);

    let peer_id = vault::get_peerid();

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
            tokio::time::sleep(Duration::from_secs(20)).await;
        }
    });

    let connection_clone = connection.clone();
    tokio::spawn(async move {
        loop {
            let (_, mut quic_recv) = connection_clone.accept_bi().await.unwrap();
            let buffer = quic_recv.read_to_end(1024 * 1024).await.unwrap();
            info!("QUIC Received:\n{}", String::from_utf8_lossy(&*buffer));
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

pub async fn get_client_connection(bootstrap_node: String) -> anyhow::Result<Connection> {
    let (certs, _key) = get_certificate().await?;

    let mut roots = rustls::RootCertStore::empty();

    for ele in certs {
        roots.add(ele)?;
    }

    let mut client_crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    let client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));
    let mut endpoint = Endpoint::client(SocketAddr::from_str("127.0.0.1:0").unwrap())?;
    endpoint.set_default_client_config(client_config);

    let server_addr: SocketAddr = bootstrap_node.parse()?;
    let conn = endpoint
        .connect(server_addr, "localhost")?
        .await
        .map_err(|e| anyhow!("failed to connect: {}", e))?;
    Ok(conn)
}

pub async fn send(
    to_peer_id: String,
    func: String,
    data: Vec<u8>,
    bootstrap_node: String,
) -> anyhow::Result<Vec<u8>> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    // 建立 QUIC 连接
    let connection = get_client_connection(bootstrap_node).await?;

    let remote_address = connection.remote_address();
    let stable_id = connection.stable_id();
    info!("established connection: {remote_address:#?},{stable_id:#?}");
    let connection = Arc::new(connection);

    let connection_clone = connection.clone();
    let local_peer_id = get_peerid();
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
        // 等待接收一个新的双向流
        let buffer = quic_recv.read_to_end(1024 * 1024).await.unwrap();
        println!("QUIC Received:\n{}", String::from_utf8_lossy(&*buffer));
        if tx.send(buffer).is_err() {
            println!("Receiver closed");
            return;
        }
    });
    let message = match rx.await {
        Ok(r) => r,
        Err(e) => {
            return Err(anyhow!("QUIC Received Error:\n{:?}", e));
        }
    };
    println!(
        "Channel Received message: {}",
        String::from_utf8_lossy(&message)
    );
    let data: SenderData = match serde_json::from_slice(&*message) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("QUIC Received Error:\n{:?}", e);
            return Err(anyhow!("QUIC Received Error:\n{:?}", e));
        }
    };
    return Ok(data.data);
}
