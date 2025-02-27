use anyhow::anyhow;
use anyhow::Result;
use dashmap::DashMap;
use lazy_static::lazy_static;
use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use quinn::rustls::server::WebPkiClientVerifier;
use quinn::{
    crypto::rustls::QuicServerConfig,
    rustls::{self},
    RecvStream, SendStream,
};
use quinn::{IdleTimeout, ServerConfig, TransportConfig, VarInt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tracing::{error, info, info_span, Instrument};

use crate::ca;
use crate::p2p::ALPN_QUIC_HTTP;

use super::Action;

lazy_static! {
    static ref Session: DashMap<String, Arc<quinn::Connection>> = DashMap::new();
    static ref REQ_ID_MAP: DashMap<String, Arc<quinn::Connection>> = DashMap::new();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReceiveData {
    pub from: String,
    pub data: Vec<u8>,
    pub func: String,
    pub action: Action,
    pub to: String,
    pub req_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SenderData {
    pub from: String,
    pub data: Vec<u8>,
    pub func: String,
    pub err: String,
    pub to: String,
    pub req_id: String,
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
    let span = info_span!(
        "connection",
        remote = %connection.remote_address(),
        protocol = %connection
            .handshake_data()
            .unwrap()
            .downcast::<quinn::crypto::rustls::HandshakeData>().unwrap()
            .protocol
            .map_or_else(|| "<none>".into(), |x| String::from_utf8_lossy(&x).into_owned())
    );
    async {
        let remote_address = connection.remote_address();
        let local_ip = connection.local_ip().unwrap();
        let stable_id = connection.stable_id();
        info!("established connection: {remote_address:#?},{local_ip:#?},{stable_id:#?}");
        let connection = Arc::new(connection);

        // Each stream initiated by the client constitutes a new request.
        loop {
            let connection_clone = connection.clone();
            let stream = connection_clone.accept_bi().await;
            let stream = match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("connection closed");
                    return Ok(());
                }
                Err(e) => {
                    info!("connection error:{}", e);
                    return Err(e);
                }
                Ok(s) => s,
            };

            // let fut = handle_request(sender.clone(), stream.1);
            let connection_clone = connection.clone();
            let fut = handle_receive(stream.0, stream.1, connection_clone);

            tokio::spawn(
                async move {
                    if let Err(e) = fut.await {
                        error!("failed: {reason}", reason = e.to_string());
                    }
                }
                .instrument(info_span!("request")),
            );
        }
    }
    .instrument(span)
    .await?;
    Ok(())
}

async fn handle_receive(
    mut _sender: SendStream,
    mut recv: RecvStream,
    connection: Arc<quinn::Connection>,
) -> anyhow::Result<()> {
    let buffer_vec = recv.read_to_end(1024 * 10).await?;
    if buffer_vec.is_empty() {
        error!("QUIC Received is empty");
        return Ok(());
    }
    let result = String::from_utf8_lossy(&buffer_vec);

    let data: ReceiveData = match serde_json::from_str(&result) {
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
            Session.insert(data.from.clone(), connection.clone());
            let sender_data = SenderData {
                from: "".to_string(),
                data: "ok".as_bytes().to_vec(),
                func: data.func.clone(),
                err: "".to_string(),
                to: data.from.to_string(),
                req_id: data.req_id,
            };
            let json = serde_json::to_string(&sender_data)?;
            let (mut quic_send, _) = connection.clone().open_bi().await?;
            quic_send.write_all(json.as_bytes()).await?;
            quic_send.finish()?;
        }
        Action::Send => {
            let connection = match Session.get(data.to.as_str()) {
                None => {
                    error!("Failed to find connection to {}", data.to);
                    return Err(anyhow!("Failed to find connection to {}", data.to));
                }
                Some(conn) => conn,
            };

            let sender_data = SenderData {
                from: data.from.to_string(),
                data: data.data,
                func: data.func.clone(),
                err: "".to_string(),
                to: data.to.to_string(),
                req_id: data.req_id,
            };
            let json = serde_json::to_string(&sender_data)?;
            let (mut send, _) = connection.open_bi().await?;
            send.write_all(json.as_bytes()).await?;
            send.finish()?;
        }
        Action::Call => {
            {
                let connection_to = match Session.get(data.to.as_str()) {
                    None => {
                        error!("Failed to find connection to {}", data.to);
                        return Err(anyhow!("Failed to find connection to {}", data.to));
                    }
                    Some(conn) => conn,
                };
                let sender_data = SenderData {
                    from: data.from.to_string(),
                    data: data.data,
                    func: data.func.clone(),
                    err: "".to_string(),
                    to: data.to.to_string(),
                    req_id: data.req_id.clone(),
                };
                let json = serde_json::to_string(&sender_data)?;
                let (mut send, _) = connection_to.open_bi().await?;
                send.write_all(json.as_bytes()).await?;
                send.finish()?;
            }
            let from_connection = connection;
            REQ_ID_MAP.insert(data.req_id.to_string(), from_connection.clone());
            Session.insert(data.from.clone(), from_connection.clone());
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
                let sender_data = SenderData {
                    from: data.from.to_string(),
                    data: data.data,
                    func: data.func.clone(),
                    err: "".to_string(),
                    to: data.to.to_string(),
                    req_id: data.req_id.clone(),
                };
                let json = serde_json::to_string(&sender_data)?;
                let (mut send, _) = connection.open_bi().await?;
                send.write_all(json.as_bytes()).await?;
                send.finish()?;
            }
            REQ_ID_MAP.remove(data.req_id.as_str());
        }
    }

    {
        let peers: Vec<String> = Session.iter().map(|entry| entry.key().clone()).collect();
        info!("Online peers num: {}", peers.len());
        for x in peers {
            info!("Online peer: {}", x.to_string());
        }
    }

    Ok(())
}

///Relay
pub async fn get_root_certificate_from_vault(
) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let cert = ca::server::get_root_cert_der().await;
    let key = ca::server::get_root_key_der().await;

    Ok((vec![cert], key))
}
