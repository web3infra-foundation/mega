use super::Action;
use crate::nostr::client_message::{ClientMessage, SubscriptionId};
use crate::nostr::event::{EventId, NostrEvent};
use crate::nostr::relay_message::RelayMessage;
use crate::nostr::tag::{Tag, TagKind};
use crate::nostr::Req;
use crate::p2p::ALPN_QUIC_HTTP;
use crate::p2p::{GitCloneHeader, RequestData};
use crate::p2p::{LFSHeader, ResponseData};
use crate::util::{get_peer_id_from_identifier, get_utc_timestamp};
use crate::{ca, Node, RepoInfo};
use anyhow::anyhow;
use anyhow::Result;
use callisto::{relay_node, relay_nostr_event, relay_nostr_req, relay_repo_info};
use dashmap::DashMap;
use jupiter::storage::Storage;
use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use quinn::rustls::server::WebPkiClientVerifier;
use quinn::{
    crypto::rustls::QuicServerConfig,
    rustls::{self},
    Connection,
};
use quinn::{IdleTimeout, ServerConfig, TransportConfig, VarInt};
use std::collections::HashSet;
use std::time::Duration;
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tokio::sync::{mpsc, OnceCell};
use tracing::{error, info};
use uuid::Uuid;
use vault::integration::VaultCore;

#[derive(Clone)]
pub struct P2PRelay {
    pub storage: Storage,
    pub vault: VaultCore,

    msg_connection_map: DashMap<String, Arc<Connection>>,
    git_objects_connection_map: DashMap<String, Arc<Connection>>,
    lfs_connection_map: DashMap<String, Arc<Connection>>,
    req_id_map: DashMap<String, Arc<Connection>>,
    nostr_event_queue: OnceCell<mpsc::Sender<(String, NostrEvent)>>,
}

impl P2PRelay {
    pub fn new(storage: Storage, vault: VaultCore) -> Self {
        Self {
            storage,
            vault,
            msg_connection_map: DashMap::new(),
            git_objects_connection_map: DashMap::new(),
            lfs_connection_map: DashMap::new(),
            req_id_map: DashMap::new(),
            nostr_event_queue: OnceCell::default(),
        }
    }

    pub fn wrapped_relay(&self) -> Arc<Self> {
        Arc::new(self.clone())
    }

    pub async fn run(&self, host: String, port: u16) -> Result<()> {
        let server_config = self.get_server_config().await?;
        let addr = format!("{}:{}", host, port);
        let endpoint =
            quinn::Endpoint::server(server_config, SocketAddr::from_str(addr.as_str()).unwrap())?;
        info!("Quic server listening on udp {}", endpoint.local_addr()?);

        //Nostr event sender channel
        let wrapped = self.wrapped_relay();
        let (tx, mut rx) = mpsc::channel(32);
        self.nostr_event_queue.set(tx)?;

        tokio::spawn(async move {
            while let Some((peer_id, nostr_event)) = rx.recv().await {
                wrapped
                    .clone()
                    .send_nostr_event(peer_id, nostr_event)
                    .await
                    .unwrap();
            }
        });

        let wrapped = self.wrapped_relay();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                wrapped.clone().check_node_status().await;
            }
        });

        while let Some(conn) = endpoint.accept().await {
            {
                info!("accepting connection");

                let wrapped = self.wrapped_relay();
                tokio::spawn(async move {
                    if let Err(e) = wrapped.handle_connection(conn).await {
                        error!("connection failed: {reason}", reason = e.to_string());
                        // remove_close_connection();
                    }
                });
            }
        }

        Ok(())
    }

    pub async fn get_server_config(&self) -> Result<ServerConfig> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let (certs, key) = self.get_root_certificate_from_vault()?;

        let mut roots = rustls::RootCertStore::empty();
        for c in &certs {
            roots.add(c.clone())?;
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

    async fn handle_connection(&self, conn: quinn::Incoming) -> Result<()> {
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
                self.msg_connection_map
                    .insert(key.to_string(), connection.clone());
                self.msg_handle_receive(connection.clone()).await?;
            }
            "REQUEST_GIT_CLONE" => {
                self.git_objects_connection_map
                    .insert(key.to_string(), connection.clone());
            }
            "RESPONSE_GIT_CLONE" => {
                self.git_clone_handle_receive(connection.clone()).await?;
            }
            "REQUEST_LFS" => {
                self.lfs_connection_map
                    .insert(key.to_string(), connection.clone());
            }
            "RESPONSE_LFS" => {
                self.lfs_handle_receive(connection.clone()).await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn _remove_close_connection(&self) {
        self.msg_connection_map
            .retain(|_, v| v.close_reason().is_some());
        self.git_objects_connection_map
            .retain(|_, v| v.close_reason().is_some());
        self.lfs_connection_map
            .retain(|_, v| v.close_reason().is_some());
        self.req_id_map.retain(|_, v| v.close_reason().is_some());
    }

    async fn check_node_status(self: Arc<Self>) {
        let relay_storage = self.storage.relay_storage().clone();
        let nodes = relay_storage.get_all_node().await.unwrap();
        for n in nodes {
            let now = get_utc_timestamp();
            if now - n.last_online_time > 60_000 {
                let mut node = n.clone();
                node.online = false;
                if let Err(e) = relay_storage.update_node(node).await {
                    error!("Failed to update node: {:?}", e);
                }
            }
        }
    }

    async fn msg_handle_receive(&self, connection: Arc<Connection>) -> Result<()> {
        loop {
            let relay_storage = self.storage.relay_storage().clone();
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
                    error!("QUIC Received Error:{:?}", e);
                    return Err(anyhow!("QUIC Received Error:{:?}", e));
                }
            };
            info!(
                "QUIC Received Message from[{}], Action[{}]",
                data.from, data.action
            );
            match data.action {
                Action::Ping => {
                    let storage = relay_storage.clone();

                    let node = relay_node::Model {
                        peer_id: data.from.clone(),
                        r#type: "mega_server".to_string(),
                        online: true,
                        last_online_time: get_utc_timestamp(),
                    };
                    match storage.insert_or_update_node(node).await {
                        Ok(_) => {
                            self.send_back(data, "ok".as_bytes().to_vec(), connection_clone)
                                .await?
                        }
                        Err(_) => {
                            self.send_back_err(
                                data,
                                "Ping with error".to_string(),
                                connection_clone,
                            )
                            .await?
                        }
                    }
                }
                Action::Send => {
                    let connection = match self.msg_connection_map.get(data.to.as_str()) {
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
                        let connection_to = match self.msg_connection_map.get(data.to.as_str()) {
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
                    self.req_id_map
                        .insert(data.req_id.to_string(), from_connection.clone());
                }
                Action::Callback => {
                    {
                        let connection = match self.req_id_map.get(data.req_id.as_str()) {
                            None => {
                                error!("Failed to find connection req {}", data.req_id);
                                return Err(anyhow!(
                                    "Failed to find connection req {}",
                                    data.req_id
                                ));
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
                    self.req_id_map.remove(data.req_id.as_str());
                }

                Action::RepoShare => {
                    let repo_info: RepoInfo = serde_json::from_slice(data.data.as_slice())?;
                    let repo_info_model: relay_repo_info::Model = repo_info.clone().into();
                    let storage = relay_storage.clone();
                    match storage.insert_or_update_repo_info(repo_info_model).await {
                        Ok(_) => {
                            self.send_back(
                                data,
                                repo_info.identifier.into_bytes(),
                                connection.clone(),
                            )
                            .await?
                        }
                        Err(_) => {
                            self.send_back_err(
                                data,
                                "Repo share failed".to_string(),
                                connection.clone(),
                            )
                            .await?
                        }
                    }
                }

                Action::Nostr => {
                    info!("Nostr data:{}", String::from_utf8(data.data.clone())?);
                    let client_msg: ClientMessage =
                        match serde_json::from_slice(data.data.clone().as_slice()) {
                            Ok(client_msg) => client_msg,
                            Err(e) => {
                                let relay_msg =
                                    RelayMessage::new_ok(EventId::empty(), false, e.to_string());
                                self.send_back(
                                    data,
                                    relay_msg.as_json().as_bytes().to_vec(),
                                    connection.clone(),
                                )
                                .await?;
                                continue;
                            }
                        };
                    let relay_msg = self.nostr_handle(client_msg, data.from.clone()).await;
                    self.send_back(
                        data,
                        relay_msg.as_json().as_bytes().to_vec(),
                        connection.clone(),
                    )
                    .await?;
                }
                Action::Peers => {
                    match relay_storage.get_all_node().await {
                        Ok(peers) => {
                            let peers: Vec<Node> = peers.iter().map(|p| p.clone().into()).collect();
                            let res = serde_json::to_string(&peers)?;
                            self.send_back(data, res.into_bytes(), connection.clone())
                                .await?
                        }
                        Err(_) => {
                            self.send_back_err(
                                data,
                                "Get peers failed".to_string(),
                                connection.clone(),
                            )
                            .await?
                        }
                    };
                }

                Action::Repos => {
                    match relay_storage.get_all_repo_info().await {
                        Ok(repo_list) => {
                            let mut repo_list: Vec<RepoInfo> =
                                repo_list.iter().map(|p| p.clone().into()).collect();
                            for r in repo_list.iter_mut() {
                                if let Ok(peer_id) =
                                    get_peer_id_from_identifier(r.identifier.clone())
                                {
                                    let node = relay_storage
                                        .get_node_by_id(peer_id.as_str())
                                        .await
                                        .unwrap();
                                    if let Some(node) = node {
                                        r.peer_online = node.online;
                                    }
                                }
                            }
                            let res = serde_json::to_string(&repo_list.clone())?;
                            self.send_back(data, res.into_bytes(), connection.clone())
                                .await?
                        }
                        Err(_) => {
                            self.send_back_err(
                                data,
                                "Get repos failed".to_string(),
                                connection.clone(),
                            )
                            .await?
                        }
                    };
                }
            }

            {
                let peers: Vec<String> = self
                    .msg_connection_map
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

    async fn nostr_handle(&self, client_message: ClientMessage, from: String) -> RelayMessage {
        let relay_storage = self.storage.relay_storage().clone();
        match client_message {
            ClientMessage::Event(nostr_event) => {
                match nostr_event.verify() {
                    Ok(_) => {}
                    Err(e) => {
                        return RelayMessage::new_ok(EventId::empty(), false, e.to_string());
                    }
                }
                let relay_nostr_event: relay_nostr_event::Model =
                    match nostr_event.clone().try_into() {
                        Ok(n) => n,
                        Err(e) => {
                            return RelayMessage::new_ok(EventId::empty(), false, e.to_string());
                        }
                    };
                //save
                if relay_storage
                    .get_nostr_event_by_id(&relay_nostr_event.id)
                    .await
                    .unwrap()
                    .is_some()
                {
                    return RelayMessage::new_ok(
                        EventId::empty(),
                        false,
                        "Duplicate submission".to_string(),
                    );
                }
                relay_storage
                    .insert_nostr_event(relay_nostr_event)
                    .await
                    .unwrap();

                //Event is forwarded to subscribed nodes
                let _ = self
                    .transfer_git_event_to_subscribers(nostr_event.clone(), from)
                    .await;
                RelayMessage::new_ok(nostr_event.id, true, "ok".to_string())
            }
            ClientMessage::Req {
                subscription_id,
                filters,
            } => {
                //subscribe message
                //save
                let filters_json = serde_json::to_string(&filters).unwrap();
                let ztm_nostr_req = relay_nostr_req::Model {
                    subscription_id: subscription_id.to_string(),
                    filters: filters_json.clone(),
                    id: Uuid::new_v4().to_string(),
                };
                let req_list: Vec<relay_nostr_req::Model> = relay_storage
                    .get_all_nostr_req_by_subscription_id(&subscription_id.to_string())
                    .await
                    .unwrap();
                match req_list.iter().find(|&x| x.filters == filters_json) {
                    Some(_) => {}
                    None => {
                        relay_storage.insert_nostr_req(ztm_nostr_req).await.unwrap();
                    }
                }
                RelayMessage::new_ok(EventId::empty(), true, "ok".to_string())
            }
        }
    }

    async fn send_back(
        &self,
        request_data: RequestData,
        data: Vec<u8>,
        connection: Arc<Connection>,
    ) -> Result<()> {
        let response = ResponseData {
            from: request_data.to.to_string(),
            data,
            func: request_data.func.clone(),
            err: "".to_string(),
            to: request_data.from.to_string(),
            req_id: request_data.req_id.clone(),
        };

        let json = serde_json::to_string(&response)?;
        let (mut send, _) = connection.open_bi().await?;
        send.write_all(json.as_bytes()).await?;
        send.finish()?;
        Ok(())
    }

    async fn send_back_err(
        &self,
        request_data: RequestData,
        err: String,
        connection: Arc<Connection>,
    ) -> Result<()> {
        let response = ResponseData {
            from: request_data.to.to_string(),
            data: vec![],
            func: request_data.func.clone(),
            err,
            to: request_data.from.to_string(),
            req_id: request_data.req_id.clone(),
        };

        let json = serde_json::to_string(&response)?;
        let (mut send, _) = connection.open_bi().await?;
        send.write_all(json.as_bytes()).await?;
        send.finish()?;
        Ok(())
    }

    async fn transfer_git_event_to_subscribers(
        &self,
        nostr_event: NostrEvent,
        from: String,
    ) -> Result<()> {
        // only support p2p_uri subscription
        let mut uri = String::new();
        let relay_storage = self.storage.relay_storage().clone();
        for tag in nostr_event.clone().tags {
            if let Tag::Generic(TagKind::URI, t) = tag {
                if !t.is_empty() {
                    uri = t.first().unwrap().to_string();
                }
            }
        }
        if uri.is_empty() {
            return Ok(());
        }
        let req_list: Vec<Req> = relay_storage
            .get_all_nostr_req()
            .await
            .unwrap()
            .iter()
            .map(|x| x.clone().into())
            .collect();
        let mut subscription_id_set: HashSet<String> = HashSet::new();
        for req in req_list {
            for filter in req.clone().filters {
                if let Some(uri_vec) = filter.generic_tags.get(&TagKind::URI.to_string()) {
                    if uri_vec.is_empty() {
                        continue;
                    }
                    let req_uri = uri_vec.first().unwrap();
                    if *req_uri == uri {
                        subscription_id_set.insert(req.subscription_id.clone());
                    }
                }
            }
        }
        info!("subscription_id_set:{:?}", subscription_id_set);
        for x in subscription_id_set {
            if x == from {
                continue;
            }
            //send to queue
            let tx = self.nostr_event_queue.get().unwrap().clone();
            tx.send((x, nostr_event.clone())).await?;
        }
        Ok(())
    }

    async fn send_nostr_event(
        self: Arc<Self>,
        peer_id: String,
        nostr_event: NostrEvent,
    ) -> Result<()> {
        if let Some(conn) = self.msg_connection_map.get(peer_id.clone().as_str()) {
            if conn.close_reason().is_some() {
                return Ok(());
            }
            let data =
                RelayMessage::new_event(SubscriptionId::new(peer_id.clone()), nostr_event.clone())
                    .as_json();
            let response = ResponseData {
                from: "relay".to_string(),
                data: data.as_bytes().to_vec(),
                func: "nostr".to_string(),
                err: "".to_string(),
                to: peer_id.clone(),
                req_id: "".to_string(),
            };

            let json = serde_json::to_string(&response)?;
            let (mut send, _) = conn.open_bi().await?;
            send.write_all(json.as_bytes()).await?;
            send.finish()?;
            info!(
                "Send nostr evnet[{}] to {} success",
                nostr_event.id.inner(),
                peer_id
            );
        }
        Ok(())
    }

    async fn git_clone_handle_receive(&self, connection: Arc<Connection>) -> Result<()> {
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

        if let Some(target_conn) = self.git_objects_connection_map.get(&key) {
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

    async fn lfs_handle_receive(&self, connection: Arc<Connection>) -> Result<()> {
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

        if let Some(target_conn) = self.lfs_connection_map.get(&key) {
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
    pub fn get_root_certificate_from_vault(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let cert = ca::server::get_root_cert_der(&self.vault);
        let key = ca::server::get_root_key_der(&self.vault);

        Ok((vec![cert], key))
    }
}
