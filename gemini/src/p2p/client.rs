use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use quinn::crypto::rustls::QuicClientConfig;
use quinn::rustls::pki_types::pem::PemObject;
use quinn::rustls::pki_types::CertificateDer;
use quinn::rustls::pki_types::PrivateKeyDer;
use quinn::Connection;
use quinn::{rustls, ClientConfig, Endpoint};
use std::result::Result::Ok;
use tokio::sync::mpsc;
use tracing::error;
use tracing::info;
use uuid::Uuid;
use vault::get_peerid;

use crate::ca;
use crate::p2p::Action;
use crate::p2p::RequestData;
use crate::p2p::ResponseData;

use super::ALPN_QUIC_HTTP;

struct MsgSingletonConnection {
    conn: Arc<quinn::Connection>,
}
static INSTANCE: OnceLock<MsgSingletonConnection> = OnceLock::new();

impl MsgSingletonConnection {
    fn new(conn: Arc<quinn::Connection>) -> Self {
        MsgSingletonConnection { conn }
    }

    pub fn init(conn: Arc<quinn::Connection>) {
        INSTANCE
            .set(Self::new(conn))
            .unwrap_or_else(|_| panic!("Singleton already initialized!"));
    }

    pub fn instance() -> &'static Self {
        INSTANCE
            .get()
            .expect("Singleton not initialized. Call Singleton::init() first.")
    }

    pub fn get_connection() -> Arc<quinn::Connection> {
        MsgSingletonConnection::instance().conn.clone()
    }
}

pub async fn run(bootstrap_node: String) -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let connection = get_client_connection(bootstrap_node.clone()).await?;

    let connection = Arc::new(connection);
    MsgSingletonConnection::init(connection.clone());

    let (tx, mut rx) = mpsc::channel(8);

    let peer_id = vault::get_peerid().await;

    tokio::spawn(async move {
        // Register msg connection to relay
        let connection_clone = MsgSingletonConnection::get_connection();
        let (mut send, _) = connection_clone.open_bi().await.unwrap();
        send.write_all(format!("{}|{}", peer_id.clone(), "MSG").as_bytes())
            .await
            .unwrap();

        loop {
            let connection_clone = MsgSingletonConnection::get_connection();
            let (mut quic_send, _) = connection_clone.open_bi().await.unwrap();

            let ping = RequestData {
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
        let data: ResponseData = match serde_json::from_slice(&message) {
            Ok(data) => data,
            Err(e) => {
                error!("QUIC Received Error:\n{:?}", e);
                continue;
            }
        };
        info!("Channel Received message: {:?}", data);
        match data.func.as_str() {
            "response_file" => {
                let path = String::from_utf8(data.data)?;
                response_file(bootstrap_node.clone(), path, data.from).await?;
            }
            "" => {}
            _ => {
                error!("Unsupported function");
            }
        }
    }

    Ok(())
}

pub async fn get_client_connection(bootstrap_node: String) -> anyhow::Result<Connection> {
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

    let server_addr: SocketAddr = bootstrap_node.parse()?;
    let connection = endpoint
        .connect(server_addr, "localhost")?
        .await
        .map_err(|e| anyhow!("failed to connect: {}", e))?;

    let remote_address = connection.remote_address();
    let stable_id = connection.stable_id();
    info!("Established connection: {remote_address:#?},{stable_id:#?}");
    Ok(connection)
}

pub async fn send(to_peer_id: String, func: String, data: Vec<u8>) -> Result<Vec<u8>> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let connection = MsgSingletonConnection::get_connection();

    let connection_clone = connection.clone();
    let local_peer_id = get_peerid().await;
    tokio::spawn(async move {
        let (mut sender, _) = connection_clone.open_bi().await.unwrap();
        let send = RequestData {
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
    let data: ResponseData = serde_json::from_slice(&message)?;
    Ok(data.data)
}

pub async fn request_file(bootstrap_node: String, path: String, to_peer_id: String) -> Result<()> {
    // Register file connection to relay
    let file_connection = get_client_connection(bootstrap_node).await?;
    let (mut file_sender, mut _file_receiver) = file_connection.open_bi().await?;
    let peer_id = get_peerid().await;
    file_sender
        .write_all(format!("{}-{}|{}", peer_id.clone(), to_peer_id, "REQUEST_FILE").as_bytes())
        .await?;
    file_sender.finish()?;

    //send file request msg via msg connection
    let (mut msg_sender, _) = MsgSingletonConnection::get_connection().open_bi().await?;

    let send = RequestData {
        from: get_peerid().await,
        data: path.as_bytes().to_vec(),
        func: "response_file".to_string(),
        action: Action::Call,
        to: to_peer_id.to_string(),
        req_id: Uuid::new_v4().into(),
    };
    let json = serde_json::to_string(&send)?;
    msg_sender.write_all(json.as_bytes()).await?;
    msg_sender.finish()?;

    //recieve file header -> {target_peer_id}|{from_peer_id}|{file_path}
    let (mut _file_sender, mut file_receiver) = file_connection.accept_bi().await?;
    let mut header_buf = [0u8; 256];
    let len = file_receiver.read(&mut header_buf).await.unwrap().unwrap();
    let header = String::from_utf8_lossy(&header_buf[..len]);

    let parts: Vec<&str> = header.splitn(3, '|').collect();
    let (target_id, from, file_path) = (parts[0], parts[1], parts[2]);
    if target_id != peer_id {
        bail!("Invalid File Connection stream,target_id != peer_id")
    }
    info!("Receive file response from [{}], path:{}", from, file_path);

    //Receive file content
    let mut file = tokio::fs::File::create("file_download").await.unwrap();
    let (mut _file_sender, mut file_receiver) = file_connection.accept_bi().await?;
    tokio::io::copy(&mut file_receiver, &mut file)
        .await
        .unwrap();
    info!("File download successfully: {}", file_path);

    Ok(())
}

pub async fn response_file(bootstrap_node: String, path: String, to_peer_id: String) -> Result<()> {
    // Register file connection to relay
    let file_connection = get_client_connection(bootstrap_node).await?;
    let (mut file_sender, mut _file_receiver) = file_connection.open_bi().await?;
    let peer_id = get_peerid().await;
    file_sender
        .write_all(format!("{}-{}|{}", peer_id.clone(), to_peer_id, "REPONSE_FILE").as_bytes())
        .await?;
    file_sender.finish()?;

    //send file header-> -> {target_peer_id}|{from_peer_id}|{file_path}
    let (mut file_sender, mut _file_receiver) = file_connection.open_bi().await?;
    file_sender
        .write_all(format!("{}|{}|{}", to_peer_id, peer_id.clone(), path).as_bytes())
        .await?;
    file_sender.finish()?;

    //send file content
    let (mut file_sender, mut _file_receiver) = file_connection.open_bi().await?;
    let mut file = tokio::fs::File::open("file_request.exe").await?;
    tokio::io::copy(&mut file, &mut file_sender).await?;
    file_sender.finish()?;
    Ok(())
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
