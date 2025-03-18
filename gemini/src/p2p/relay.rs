use anyhow::anyhow;
use anyhow::Result;
use dashmap::DashMap;
use lazy_static::lazy_static;
use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use quinn::rustls::server::WebPkiClientVerifier;
use quinn::{
    crypto::rustls::QuicServerConfig,
    rustls::{self},
    Connection,
};
use quinn::{IdleTimeout, ServerConfig, TransportConfig, VarInt};
use std::time::Duration;
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::p2p::ALPN_QUIC_HTTP;
use crate::p2p::{GitCloneHeader, RequestData};
use crate::p2p::{LFSHeader, ResponseData};
use crate::{ca, RepoInfo};

use super::Action;

lazy_static! {
    static ref MSG_CONNECTION_MAP: DashMap<String, Arc<Connection>> = DashMap::new();
    static ref GIT_OBJECTS_CONNECTION_MAP: DashMap<String, Arc<Connection>> = DashMap::new();
    static ref LFS_CONNECTION_MAP: DashMap<String, Arc<Connection>> = DashMap::new();
    static ref REQ_ID_MAP: DashMap<String, Arc<Connection>> = DashMap::new();
    static ref REPO_LIST: RwLock<Vec<RepoInfo>> = RwLock::new(Vec::new());
}

pub async fn run(host: String, port: u16) -> Result<()> {
    let server_config = get_server_config().await?;
    let addr = format!("{}:{}", host, port);
    let endpoint =
        quinn::Endpoint::server(server_config, SocketAddr::from_str(addr.as_str()).unwrap())?;
    info!("Quic server listening on udp {}", endpoint.local_addr()?);

    while let Some(conn) = endpoint.accept().await {
        {
            info!("accepting connection");
            let fut = handle_connection(conn);
            tokio::spawn(async move {
                if let Err(e) = fut.await {
                    error!("connection failed: {reason}", reason = e.to_string())
                }
            });
        }
    }

    Ok(())
}

pub async fn get_server_config() -> Result<ServerConfig> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let (certs, key) = get_root_certificate_from_vault().await?;

    let mut roots = rustls::RootCertStore::empty();
    for c in certs.clone() {
        roots.add(c)?;
    }

    let client_verifier = WebPkiClientVerifier::builder(roots.into())
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)?;
    server_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    server_crypto.max_early_data_size = u32::MAX;

    let mut server_config =
        quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));

    let mut transport_config = TransportConfig::default();
    transport_config.max_idle_timeout(Some(IdleTimeout::from(VarInt::from_u32(300_000))));
    transport_config.keep_alive_interval(Some(Duration::from_secs(15)));
    server_config.transport_config(transport_config.into());

    Ok(server_config)
}

async fn handle_connection(conn: quinn::Incoming) -> Result<()> {
    let connection = conn.await?;

    let remote_address = connection.remote_address();
    let local_ip = connection.local_ip().unwrap();
    let stable_id = connection.stable_id();
    info!("Established connection: {remote_address:#?},{local_ip:#?},{stable_id:#?}");
    let connection = Arc::new(connection);

    let (mut _send, mut recv) = connection.accept_bi().await.unwrap();
    let mut buf = [0u8; 1024];
    let len = recv.read(&mut buf).await.unwrap().unwrap();
    let registration = String::from_utf8_lossy(&buf[..len]);

    //register: key |ConnectionType (MSG/REQUEST_GIT_CLONE/REQUEST_LFS)
    let parts: Vec<&str> = registration.split('|').collect();
    let (key, connection_type) = (parts[0], parts[1]);
    info!("Key:{}, Connection_type:{}", key, connection_type);
    match connection_type {
        "MSG" => {
            MSG_CONNECTION_MAP.insert(key.to_string(), connection.clone());
            msg_handle_receive(connection.clone()).await?;
        }
        "REQUEST_GIT_CLONE" => {
            GIT_OBJECTS_CONNECTION_MAP.insert(key.to_string(), connection.clone());
        }
        "RESPONSE_GIT_CLONE" => {
            git_clone_handle_receive(connection.clone()).await?;
        }
        "REQUEST_LFS" => {
            LFS_CONNECTION_MAP.insert(key.to_string(), connection.clone());
        }
        "RESPONSE_LFS" => {
            lfs_handle_receive(connection.clone()).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn msg_handle_receive(connection: Arc<quinn::Connection>) -> anyhow::Result<()> {
    loop {
        let connection_clone = connection.clone();
        let stream = connection_clone.accept_bi().await;
        let (_sender, mut recv) = match stream {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                info!("connection closed");
                return Ok(());
            }
            Err(e) => {
                info!("connection error:{}", e);
                return Err(e.into());
            }
            Ok(s) => s,
        };
        let buffer_vec = recv.read_to_end(1024 * 10).await?;
        if buffer_vec.is_empty() {
            error!("QUIC Received is empty");
            return Ok(());
        }
        let result = String::from_utf8_lossy(&buffer_vec);

        let data: RequestData = match serde_json::from_str(&result) {
            Ok(data) => data,
            Err(e) => {
                error!("QUIC Received Error:\n{:?}", e);
                return Err(anyhow!("QUIC Received Error:\n{:?}", e));
            }
        };
        info!(
            "QUIC Received Message from[{}], Action[{}]:\n",
            data.from, data.action
        );
        match data.action {
            Action::Ping => {
                let sender_data = ResponseData {
                    from: "relay".to_string(),
                    data: "ok".as_bytes().to_vec(),
                    func: data.func.clone(),
                    err: "".to_string(),
                    to: data.from.to_string(),
                    req_id: data.req_id,
                };
                let json = serde_json::to_string(&sender_data)?;
                let (mut quic_send, _) = connection_clone.clone().open_bi().await?;
                quic_send.write_all(json.as_bytes()).await?;
                quic_send.finish()?;
            }
            Action::Send => {
                let connection = match MSG_CONNECTION_MAP.get(data.to.as_str()) {
                    None => {
                        error!("Failed to find connection to {}", data.to);
                        return Err(anyhow!("Failed to find connection to {}", data.to));
                    }
                    Some(conn) => conn,
                };

                let reponse = ResponseData {
                    from: data.from.to_string(),
                    data: data.data,
                    func: data.func.clone(),
                    err: "".to_string(),
                    to: data.to.to_string(),
                    req_id: data.req_id,
                };
                let json = serde_json::to_string(&reponse)?;
                let (mut send, _) = connection.open_bi().await?;
                send.write_all(json.as_bytes()).await?;
                send.finish()?;
            }
            Action::Call => {
                {
                    let connection_to = match MSG_CONNECTION_MAP.get(data.to.as_str()) {
                        None => {
                            error!("Failed to find connection to {}", data.to);
                            return Err(anyhow!("Failed to find connection to {}", data.to));
                        }
                        Some(conn) => conn,
                    };
                    let response = ResponseData {
                        from: data.from.to_string(),
                        data: data.data,
                        func: data.func.clone(),
                        err: "".to_string(),
                        to: data.to.to_string(),
                        req_id: data.req_id.clone(),
                    };
                    let json = serde_json::to_string(&response)?;
                    let (mut send, _) = connection_to.open_bi().await?;
                    send.write_all(json.as_bytes()).await?;
                    send.finish()?;
                }
                let from_connection = connection_clone;
                REQ_ID_MAP.insert(data.req_id.to_string(), from_connection.clone());
            }
            Action::Callback => {
                {
                    let connection = match REQ_ID_MAP.get(data.req_id.as_str()) {
                        None => {
                            error!("Failed to find connection req {}", data.req_id);
                            return Err(anyhow!("Failed to find connection req {}", data.req_id));
                        }
                        Some(conn) => conn,
                    };
                    let response = ResponseData {
                        from: data.from.to_string(),
                        data: data.data,
                        func: data.func.clone(),
                        err: "".to_string(),
                        to: data.to.to_string(),
                        req_id: data.req_id.clone(),
                    };
                    let json = serde_json::to_string(&response)?;
                    let (mut send, _) = connection.open_bi().await?;
                    send.write_all(json.as_bytes()).await?;
                    send.finish()?;
                }
                REQ_ID_MAP.remove(data.req_id.as_str());
            }

            Action::RepoShare => {
                let response = ResponseData {
                    from: "relay".to_string(),
                    data: "ok".as_bytes().to_vec(),
                    func: data.func.clone(),
                    err: "".to_string(),
                    to: data.from.to_string(),
                    req_id: data.req_id.clone(),
                };
                //return to from node
                let connection = match MSG_CONNECTION_MAP.get(data.from.as_str()) {
                    None => {
                        error!("Failed to find connection to {}", data.to);
                        return Err(anyhow!("Failed to find connection to {}", data.to));
                    }
                    Some(conn) => conn,
                };
                let json = serde_json::to_string(&response)?;
                let (mut send, _) = connection.open_bi().await?;
                send.write_all(json.as_bytes()).await?;
                send.finish()?;
            }
        }

        {
            let peers: Vec<String> = MSG_CONNECTION_MAP
                .iter()
                .map(|entry| entry.key().clone())
                .collect();
            info!("Online peers num: {}", peers.len());
            for x in peers {
                info!("Online peer: {}", x.to_string());
            }
        }
    }
}

async fn git_clone_handle_receive(connection: Arc<Connection>) -> Result<()> {
    let connection_clone: Arc<Connection> = connection.clone();
    let (_file_sender, mut file_receiver) = connection_clone.accept_bi().await?;

    //read header
    let mut header_buf = [0u8; 4096];
    let len = file_receiver.read(&mut header_buf).await?.unwrap();
    let header = String::from_utf8_lossy(&header_buf[..len]);
    let header: GitCloneHeader = serde_json::from_str(&header)?;
    let (target_id, from, git_path) = (header.target, header.from, header.git_path);
    info!(
        "File handle receive, target_id:{}, from:{}, file_path:{}",
        target_id, from, git_path
    );
    let key = format!("git-clone-{}-{}", target_id, from);

    if let Some(target_conn) = GIT_OBJECTS_CONNECTION_MAP.get(&key) {
        info!("Find target connection to {}", target_id);
        //header data
        info!("Send git clone header to {}", target_id);
        let (mut target_sender, _) = target_conn.open_bi().await?;
        target_sender.write_all(&header_buf[..len]).await?;
        target_sender.finish()?;

        //git objects data
        info!("Send git clone objects to {}", target_id);
        let (mut target_sender, _) = target_conn.open_bi().await?;
        let (_file_sender, mut file_receiver) = connection_clone.accept_bi().await?;
        tokio::io::copy(&mut file_receiver, &mut target_sender).await?;
        target_sender.finish()?;

        //send finish to from peer
        let (mut sender, _) = connection_clone.open_bi().await?;
        sender.write_all("finish".as_bytes()).await?;
        sender.finish()?;
        info!("Finish git clone to provider:{}", from);
    } else {
        connection_clone.close(VarInt::from_u32(1), "Cannot find target peer".as_bytes());
    }
    Ok(())
}

async fn lfs_handle_receive(connection: Arc<Connection>) -> Result<()> {
    let connection_clone: Arc<Connection> = connection.clone();
    let (_file_sender, mut file_receiver) = connection_clone.accept_bi().await?;

    //read header
    let mut header_buf = [0u8; 4096];
    let len = file_receiver.read(&mut header_buf).await?.unwrap();
    let header = String::from_utf8_lossy(&header_buf[..len]);
    let header: LFSHeader = serde_json::from_str(&header)?;
    let (target_id, from, oid, size) = (header.target, header.from, header.oid, header.size);
    info!(
        "LFS handle receive, target_id:{}, from:{}, oid:{}: size:{}",
        target_id, from, oid, size
    );
    let key = format!("lfs-{}-{}", target_id, from);

    if let Some(target_conn) = LFS_CONNECTION_MAP.get(&key) {
        info!("Find target connection to {}", target_id);
        //header data
        info!("Send lfs header to {}", target_id);
        let (mut target_sender, _) = target_conn.open_bi().await?;
        target_sender.write_all(&header_buf[..len]).await?;
        target_sender.finish()?;

        //lfs data
        info!("Send lfs data to {}", target_id);
        let (mut target_sender, _) = target_conn.open_bi().await?;
        let (_file_sender, mut file_receiver) = connection_clone.accept_bi().await?;
        tokio::io::copy(&mut file_receiver, &mut target_sender).await?;
        target_sender.finish()?;

        //send finish to from peer
        let (mut sender, _) = connection_clone.open_bi().await?;
        sender.write_all("finish".as_bytes()).await?;
        sender.finish()?;
        info!("Finish lfs to provider:{}", from);
    } else {
        connection_clone.close(VarInt::from_u32(1), "Cannot find target peer".as_bytes());
    }
    Ok(())
}

//Relay
pub async fn get_root_certificate_from_vault(
) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let cert = ca::server::get_root_cert_der().await;
    let key = ca::server::get_root_key_der().await;

    Ok((vec![cert], key))
}
