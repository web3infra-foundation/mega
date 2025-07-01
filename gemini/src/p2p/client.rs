use std::borrow::Cow;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use callisto::{git_repo, import_refs, lfs_objects};
use ceres::lfs::handler;
use ceres::lfs::handler::lfs_download_object;
use ceres::lfs::lfs_structs::RequestObject;
use ceres::pack::import_repo::ImportRepo;
use ceres::pack::PackHandler;
use ceres::protocol::repo::Repo;
use common::utils::generate_id;
use dashmap::DashMap;
use futures_util::{StreamExt, TryStreamExt};
use jupiter::storage::Storage;
use mercury::internal::object::types::ObjectType;
use mercury::internal::pack::Pack;
use quinn::crypto::rustls::QuicClientConfig;
use quinn::Connection;
use quinn::{rustls, ClientConfig, Endpoint};
use std::result::Result::Ok;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use tokio::sync::{mpsc, Mutex};
use tokio::{join, time};
use tokio_util::io::ReaderStream;
use tracing::error;
use tracing::info;
use uuid::Uuid;
use vault::integration::vault_core::VaultCore;

use super::{LFSHeader, ALPN_QUIC_HTTP};
use crate::nostr::client_message::{ClientMessage, Filter, SubscriptionId};
use crate::nostr::event::NostrEvent;
use crate::nostr::relay_message::RelayMessage;
use crate::nostr::GitEvent;
use crate::p2p::RequestData;
use crate::p2p::ResponseData;
use crate::p2p::{Action, GitCloneHeader};
use crate::util::{
    get_git_model_by_path, get_path_from_identifier, get_peer_id_from_identifier, get_repo_path,
    parse_pointer_data, repo_path_to_identifier,
};
use crate::{Node, RepoInfo};

type ReqSenderType = Sender<Vec<u8>>;

#[derive(Clone)]
pub struct P2PClient {
    pub storage: Storage,
    pub vault: VaultCore,

    req_senders: DashMap<String, Arc<Mutex<Option<ReqSenderType>>>>,
    bootstrap_node: OnceLock<String>,
    connection: OnceLock<Arc<Connection>>,

    pub http_client: reqwest::Client,
    pub peer_id: Arc<str>,
}

impl P2PClient {
    pub fn new(storage: Storage, vault: VaultCore) -> Self {
        let peer_id = vault.load_nostr_peerid();
        P2PClient {
            storage,
            vault,
            req_senders: DashMap::new(),
            bootstrap_node: OnceLock::new(),
            connection: OnceLock::new(),
            http_client: Default::default(),

            peer_id: Arc::from(peer_id),
        }
    }

    pub fn get_bootstrap_node(&self) -> Cow<str> {
        let ref_str = self
            .bootstrap_node
            .get()
            .expect("Bootstrap node must be set before using P2P client")
            .as_str();
        Cow::Borrowed(ref_str)
    }

    pub fn get_connection(&self) -> Arc<Connection> {
        self.connection
            .get()
            .expect("Connection must be set before using P2P client")
            .clone()
    }

    /// Some methods keeps asking for a String typed peer_id, then for it.
    /// In ordinary cases, use self.peer_id directly for better performance.
    pub fn get_peer_id(&self) -> String {
        String::from(self.peer_id.as_ref())
    }

    /// Some methods are designed to be used with an Arc<P2PClient> to cross threads.
    pub fn wrapped_client(&self) -> Arc<Self> {
        Arc::new(self.clone())
    }

    pub async fn run(&self, bootstrap_node: String) -> Result<()> {
        let peer_id = self.peer_id.clone();
        let (tx, mut rx) = mpsc::channel(8);

        self.bootstrap_node
            .set(bootstrap_node.clone())
            .expect("Bootstrap node must be set only once");

        let connection = match self.new_client_connection().await {
            Ok(connection) => Arc::new(connection),
            Err(e) => {
                bail!(
                    "P2P: Connect to {} failed, {}",
                    self.get_bootstrap_node(),
                    e
                );
            }
        };

        self.connection
            .set(connection.clone())
            .expect("Connection must be set only once");

        // Register msg connection to relay
        let (mut send, _) = connection.clone().open_bi().await?;
        send.write_all(format!("{}|{}", peer_id, "MSG").as_bytes())
            .await?;
        send.finish()?;

        let client = self.wrapped_client();
        tokio::spawn(async move {
            if let Err(e) = client.run_ping_task(peer_id).await {
                error!("P2P: Ping Task Error, {}", e);
            }
        });

        let client = self.wrapped_client();
        tokio::spawn(async move {
            if let Err(e) = client.receive_quic_msg_task(tx.clone()).await {
                error!("P2P: Receive quic msg Error, {}", e);
            }
        });

        while let Some(message) = rx.recv().await {
            let client = self.wrapped_client();
            tokio::spawn(async move {
                if let Err(e) = client.handle_quic_msg_task(message).await {
                    error!("P2P: Handle quic msg Error, {}", e);
                }
            });
        }
        Ok(())
    }

    async fn new_client_connection(&self) -> Result<Connection> {
        let bootstrap_node = self
            .bootstrap_node
            .get()
            .ok_or_else(|| anyhow!("Bootstrap node must be set before using P2P client"))?;
        let (user_cert, user_key) = self.get_user_cert_from_ca(bootstrap_node).await?;
        let ca_cert = self.get_ca_cert_from_ca(bootstrap_node).await?;

        let mut roots = rustls::RootCertStore::empty();

        roots.add(ca_cert)?;

        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_client_auth_cert([user_cert].to_vec(), user_key)?;
        client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
        client_crypto.enable_early_data = true;
        let client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));
        let mut endpoint = Endpoint::client(SocketAddr::from_str("[::]:0")?)?;
        endpoint.set_default_client_config(client_config);

        let server_addr: SocketAddr = self.bootstrap_node.get().unwrap().parse()?;
        let connection = endpoint
            .connect(server_addr, "localhost")?
            .await
            .map_err(|e| anyhow!("failed to connect: {}", e))?;

        let remote_address = connection.remote_address();
        let stable_id = connection.stable_id();
        info!("Established connection: {remote_address:#?},{stable_id:#?}");
        Ok(connection)
    }

    // pub async fn call(to_peer_id: String, func: String, data: Vec<u8>) -> Result<Vec<u8>> {
    //     let (tx, rx) = tokio::sync::oneshot::channel();
    //
    //     let connection = MsgSingletonConnection::get_connection();
    //
    //     let connection_clone = connection.clone();
    //     let local_peer_id = get_peerid().await;
    //     tokio::spawn(async move {
    //         let (mut sender, _) = connection_clone.open_bi().await.unwrap();
    //         let send = RequestData {
    //             from: local_peer_id.clone(),
    //             data: data.clone(),
    //             func: func.to_string(),
    //             action: Action::Call,
    //             to: to_peer_id.to_string(),
    //             req_id: Uuid::new_v4().into(),
    //         };
    //         let json = serde_json::to_string(&send).unwrap();
    //         sender.write_all(json.as_bytes()).await.unwrap();
    //         sender.finish().unwrap();
    //     });
    //
    //     let connection_clone = connection.clone();
    //
    //     tokio::spawn(async move {
    //         let (_, mut quic_recv) = connection_clone.accept_bi().await.unwrap();
    //         let buffer = quic_recv.read_to_end(1024 * 1024).await.unwrap();
    //         info!("QUIC Received:\n{}", String::from_utf8_lossy(&buffer));
    //         if tx.send(buffer).is_err() {
    //             info!("Receiver closed");
    //         }
    //     });
    //     let message = rx.await?;
    //     let data: ResponseData = serde_json::from_slice(&message)?;
    //     Ok(data.data)
    // }

    pub async fn send(&self, to_peer_id: String, func: String, data: Vec<u8>) -> Result<()> {
        let peer_id = self.get_peer_id();
        let client = self.wrapped_client();
        let t = tokio::spawn(async move {
            let send = RequestData {
                from: peer_id,
                data: data.clone(),
                func: func.to_string(),
                action: Action::Send,
                to: to_peer_id.to_string(),
                req_id: Uuid::new_v4().into(),
            };
            if let Err(e) = client.send_request(send).await {
                error!("failed to send request: {e}");
            };
        });
        let _ = join!(t);
        Ok(())
    }

    pub async fn repo_share(&self, path: String) -> Result<String> {
        let db = self.storage.services.git_db_storage.clone();
        let client = self.wrapped_client();

        let repo: git_repo::Model =
            match get_git_model_by_path(self.storage.clone(), path.clone()).await {
                None => {
                    bail!("Repo not found: {}", path);
                }
                Some(repo) => repo,
            };
        let commit = match db.get_last_commit_by_repo_id(repo.id).await {
            Ok(commit) => commit,
            Err(e) => bail!(e),
        };
        let commit = match commit {
            Some(commit) => commit,
            None => {
                bail!("Repo commit error");
            }
        };
        let mut repo_info: RepoInfo = repo.clone().into();
        let identifier = repo_path_to_identifier(self.peer_id.as_ref(), repo.repo_path).await;
        repo_info.identifier = identifier.clone();
        repo_info.commit = commit.commit_id;
        repo_info.update_time = commit.created_at.and_utc().timestamp();
        repo_info.origin = self.get_peer_id();

        let local_peer_id = self.get_peer_id();
        let req_id: String = Uuid::new_v4().into();

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.req_senders
            .insert(req_id.clone(), Arc::new(Mutex::new(Some(tx))));
        tokio::spawn(async move {
            let send = RequestData {
                from: local_peer_id,
                data: repo_info.to_json().into_bytes(),
                func: "".to_string(),
                action: Action::RepoShare,
                to: "".to_string(),
                req_id: req_id.clone(),
            };

            if let Err(e) = client.send_request(send).await {
                error!("failed to send request: {e}");
            }
        });
        let _message = wait_rx_with_timeout(rx).await?;
        info!("Repo share success: {}", identifier);
        Ok(identifier)
    }

    pub async fn repo_clone(&self, identifier: String) -> Result<String> {
        let remote_peer_id = match get_peer_id_from_identifier(identifier.clone()) {
            Ok(p) => p,
            Err(_e) => {
                bail!("Identifier invalid");
            }
        };
        let path = match get_path_from_identifier(identifier.clone()) {
            Ok(p) => p,
            Err(_e) => {
                bail!("Identifier invalid");
            }
        };
        let path = get_repo_path(path);
        self.request_git_clone(path, remote_peer_id).await?;
        Ok(identifier.clone())
    }

    async fn request_git_clone(&self, path: String, to_peer_id: String) -> Result<()> {
        let db = self.storage.services.git_db_storage.clone();
        let model = match db.find_git_repo_exact_match(path.as_str()).await {
            Ok(model) => model,
            Err(e) => bail!(e),
        };
        if model.is_some() {
            bail!("Repo path already exists");
        }

        // Register file connection to relay
        let file_connection = self.new_client_connection().await?;
        let (mut file_sender, mut _file_receiver) = file_connection.open_bi().await?;
        file_sender
            .write_all(
                format!(
                    "git-clone-{}-{}|{}",
                    self.peer_id, to_peer_id, "REQUEST_GIT_CLONE"
                )
                .as_bytes(),
            )
            .await?;
        file_sender.finish()?;

        //send git clone request msg via msg connection
        let (mut msg_sender, _) = self.get_connection().open_bi().await?;

        let send = RequestData {
            from: self.get_peer_id(),
            data: path.as_bytes().to_vec(),
            func: "request_git_clone".to_string(),
            action: Action::Send,
            to: to_peer_id.to_string(),
            req_id: Uuid::new_v4().into(),
        };
        let json = serde_json::to_string(&send)?;
        msg_sender.write_all(json.as_bytes()).await?;
        msg_sender.finish()?;

        //receive  header
        let (_file_sender, mut file_receiver) = file_connection.accept_bi().await?;
        let mut header_buf = [0u8; 1024];
        let header = match file_receiver.read(&mut header_buf).await? {
            Some(len) => String::from_utf8_lossy(&header_buf[..len]),
            None => {
                bail!("failed to read header");
            }
        };
        let header: GitCloneHeader = serde_json::from_str(&header)?;
        let (target_id, from, git_path) = (header.target, header.from, header.git_path);
        if target_id != *self.peer_id {
            bail!("Invalid Connection stream,target_id != peer_id")
        }
        if git_path != path {
            bail!("Invalid Connection stream,target_path != request_path")
        }
        info!(
            "Receive git clone response from [{}], path:{}",
            from, git_path
        );

        //Receive git encode objects
        let (_file_sender, file_receiver) = file_connection.accept_bi().await?;

        let stream = ReaderStream::new(file_receiver).map_err(axum::Error::new);
        let repo = Repo::new(get_repo_path(path).parse()?, false);

        //decode the git objects
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let p = Pack::new(
            None,
            Some(1024 * 1024 * 1024 * 4),
            Some(self.storage.config().pack.pack_decode_cache_path.clone()),
            self.storage.config().pack.clean_cache_after_decode,
        );
        p.decode_stream(stream, sender).await;
        let mut entry_list = vec![];
        while let Some(entry) = receiver.recv().await {
            entry_list.push(entry);
        }

        // deal lfs blob
        let mut task = vec![];
        for blob in entry_list.iter().filter(|e| e.obj_type == ObjectType::Blob) {
            let oid = parse_pointer_data(&blob.data);
            if let Some(oid) = oid {
                let client = self.wrapped_client();
                let to_peer_id = to_peer_id.clone();
                let t = tokio::spawn(async move {
                    //try to download lfs
                    match client
                        .request_lfs(oid.0.to_string(), to_peer_id.clone())
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            error!("failed request lfs: {e}");
                        }
                    };
                });
                task.push(t);
            }
        }
        futures::future::join_all(task).await;
        //Save to db
        if let Err(e) = db.save_git_repo(repo.clone().into()).await {
            bail!("failed to save git repo: {}", e);
        };
        if let Err(e) = db.save_entry(repo.repo_id, entry_list).await {
            bail!("failed to save entry for repo: {}", e);
        };
        for x in header.branches {
            let r = import_refs::Model {
                id: generate_id(),
                repo_id: repo.repo_id,
                ref_name: x.ref_name,
                ref_git_id: x.ref_git_id,
                ref_type: x.ref_type.clone(),
                default_branch: x.default_branch,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
            };
            if let Err(e) = db.save_ref(repo.repo_id, r).await {
                bail!("failed to save reference: {}", e);
            }
        }

        info!(
            "Git clone from[{}] with path[{}] successfully",
            to_peer_id, git_path
        );

        Ok(())
    }

    async fn response_git_clone(&self, path: String, to_peer_id: String) -> Result<()> {
        let db = self.storage.services.git_db_storage.clone();

        let repo: Repo = match get_git_model_by_path(self.storage.clone(), path.clone()).await {
            None => {
                bail!("Repo not found: {}", path);
            }
            Some(repo) => repo.into(),
        };
        // Register file connection to relay
        let file_connection = self.new_client_connection().await?;
        let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;

        file_sender
            .write_all(
                format!(
                    "git-clone-{}-{}|{}",
                    self.peer_id, to_peer_id, "RESPONSE_GIT_CLONE"
                )
                .as_bytes(),
            )
            .await?;
        file_sender.finish()?;

        //send git clone header
        let refs = match db.get_ref(repo.repo_id).await {
            Ok(refs) => refs,
            Err(e) => bail!("failed to fetch refs: {e}"),
        };
        let header = GitCloneHeader {
            from: self.get_peer_id(),
            target: to_peer_id.clone(),
            git_path: path.clone(),
            branches: refs,
        };
        let header = serde_json::to_string(&header)?;
        let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
        file_sender.write_all(header.as_bytes()).await?;
        file_sender.finish()?;

        //send encoded git objects
        let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
        let mut receiver = self.get_encode_git_objects_by_repo(repo).await?;
        while let Some(data) = receiver.recv().await {
            file_sender.write_all(&data).await?;
        }
        file_sender.finish()?;

        //wait finish msg
        match file_connection.accept_bi().await {
            Ok((_file_sender, mut file_receiver)) => {
                file_receiver.read_to_end(1024).await?;
            }
            Err(_) => {
                info!("Git clone connection closed.");
            }
        };
        info!(
            "Send git clone data to[{}] with path[{}] successfully",
            to_peer_id, path
        );
        Ok(())
    }

    pub async fn repo_subscribe(&self, identifier: String) -> Result<()> {
        let filters = vec![Filter::new().repo_uri(identifier)];
        let subscription_id = self.get_peer_id();
        let client_req = ClientMessage::new_req(SubscriptionId::new(subscription_id), filters);

        let relay_message = self.send_nostr_msg(client_req).await?;
        info!("Subscribe repo result: {}", relay_message.as_json());
        Ok(())
    }

    pub async fn send_git_event(&self, git_event: GitEvent) -> Result<()> {
        let keypair = self.vault.load_nostr_secp_pair();
        let event = NostrEvent::new_git_event(keypair, git_event);
        let client_message = ClientMessage::new_event(event);
        let relay_message = self.send_nostr_msg(client_message).await?;
        info!("Sent git event result: {}", relay_message.as_json());
        Ok(())
    }

    pub async fn get_peers(&self) -> Result<Vec<Node>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let req_id: String = Uuid::new_v4().into();

        self.req_senders
            .insert(req_id.clone(), Arc::new(Mutex::new(Some(tx))));
        let send = RequestData {
            from: self.get_peer_id(),
            data: vec![],
            func: "".to_string(),
            action: Action::Peers,
            to: "".to_string(),
            req_id: req_id.clone(),
        };
        if let Err(e) = self.wrapped_client().send_request(send).await {
            error!("failed to get peers: {e}");
        }

        let res = wait_rx_with_timeout(rx).await?;
        let peers: Vec<Node> = serde_json::from_slice(res.as_slice())?;
        Ok(peers)
    }

    pub async fn get_repos(&self) -> Result<Vec<RepoInfo>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let req_id: String = Uuid::new_v4().into();

        self.req_senders
            .insert(req_id.clone(), Arc::new(Mutex::new(Some(tx))));
        let send = RequestData {
            from: self.get_peer_id(),
            data: vec![],
            func: "".to_string(),
            action: Action::Repos,
            to: "".to_string(),
            req_id: req_id.clone(),
        };

        if let Err(e) = self.wrapped_client().send_request(send).await {
            error!("failed to get repos: {e}");
        }
        let res = wait_rx_with_timeout(rx).await?;
        let repo_list: Vec<RepoInfo> = serde_json::from_slice(res.as_slice())?;
        Ok(repo_list)
    }

    async fn get_encode_git_objects_by_repo(&self, repo: Repo) -> Result<Receiver<Vec<u8>>> {
        let import_repo = ImportRepo {
            storage: self.storage.clone(),
            repo,
            command_list: vec![],
        };
        match import_repo.full_pack(vec![]).await {
            Ok(s) => Ok(s.into_inner()),
            Err(e) => bail!("full pack repo failed: {}", e),
        }
    }

    async fn request_lfs(self: Arc<Self>, oid: String, to_peer_id: String) -> Result<()> {
        // Register file connection to relay
        let file_connection = self.new_client_connection().await?;
        let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
        file_sender
            .write_all(format!("lfs-{}-{}|{}", self.peer_id, to_peer_id, "REQUEST_LFS").as_bytes())
            .await?;
        file_sender.finish()?;

        //send request lfs request msg via msg connection
        let (mut msg_sender, _) = self.get_connection().open_bi().await?;

        let send = RequestData {
            from: self.get_peer_id(),
            data: oid.as_bytes().to_vec(),
            func: "request_lfs".to_string(),
            action: Action::Send,
            to: to_peer_id.to_string(),
            req_id: Uuid::new_v4().into(),
        };
        let json = serde_json::to_string(&send)?;
        msg_sender.write_all(json.as_bytes()).await?;
        msg_sender.finish()?;

        //receive  header
        let (_file_sender, mut file_receiver) = file_connection.accept_bi().await?;
        let mut header_buf = [0u8; 1024];
        let len = match file_receiver.read(&mut header_buf).await? {
            Some(n) => n,
            None => bail!("failed to read header"),
        };
        let header = String::from_utf8_lossy(&header_buf[..len]);
        let header: LFSHeader = serde_json::from_str(&header)?;
        info!("LFS handle receive, {:?}", header);
        if header.target != *self.peer_id {
            bail!("Invalid Connection stream,target_id != peer_id")
        }
        if oid != header.oid {
            bail!("Invalid Connection stream,oid != header.oid")
        }
        info!(
            "Start download lfs from [{}], oid:{}, size:{}",
            header.from, header.oid, header.size
        );

        //Receive lfs data
        let (_file_sender, mut file_receiver) = file_connection.accept_bi().await?;
        let mut data: Vec<u8> = vec![];
        let mut buffer = vec![0; 1024 * 8];
        while let Ok(bytes_read) = file_receiver.read(&mut buffer).await {
            match bytes_read {
                Some(bytes_read) => {
                    data.append(&mut buffer[..bytes_read].to_vec());
                }
                None => {
                    break;
                }
            }
        }
        // let data = file_receiver.read_to_end(header.size as usize).await?;
        info!(
            "Download lfs from [{}], oid:{}, size:{} successfully",
            header.from, header.oid, header.size
        );
        let splited = self.storage.config().lfs.local.enable_split;
        let meta_to = lfs_objects::Model {
            oid: header.oid,
            size: header.size,
            exist: true,
            splited,
        };

        let res = self.storage.lfs_db_storage().new_lfs_object(meta_to).await;
        match res {
            Ok(_) => {}
            Err(e) => {
                error!("Insert lfs object failed:{}", e);
            }
        }

        // Load request parameters into struct.
        let req_obj = RequestObject {
            oid,
            ..Default::default()
        };

        let result = handler::lfs_upload_object(&self.storage.clone(), &req_obj, data).await;

        match result {
            Ok(_) => {
                info!("Upload lfs successfully",);
            }
            Err(e) => {
                error!("Upload lfs failed:{}", e);
            }
        }
        Ok(())
    }

    async fn response_lfs(self: Arc<Self>, oid: String, to_peer_id: String) -> Result<()> {
        info!("oid:{}", oid.clone());
        let result = match self
            .storage
            .lfs_db_storage()
            .get_lfs_object(oid.as_str())
            .await
        {
            Ok(m) => m,
            Err(e) => bail!(e),
        };

        let lfs_object = match result {
            None => {
                bail!("LFS not found: {}", oid);
            }
            Some(o) => o,
        };

        // Register lfs connection to relay
        let file_connection = self.new_client_connection().await?;
        let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
        file_sender
            .write_all(format!("lfs-{}-{}|{}", self.peer_id, to_peer_id, "RESPONSE_LFS").as_bytes())
            .await?;
        file_sender.finish()?;

        //send lfs header
        let header = LFSHeader {
            from: self.get_peer_id(),
            target: to_peer_id.clone(),
            oid: oid.clone(),
            size: lfs_object.size,
        };
        let header = serde_json::to_string(&header)?;
        let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
        file_sender.write_all(header.as_bytes()).await?;
        file_sender.finish()?;

        //send data
        let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
        let mut result = lfs_download_object(self.storage.clone(), oid.clone()).await?;
        while let Some(d) = result.next().await {
            match d {
                Ok(bytes_chunk) => {
                    info!("bytes_chunk:{}", bytes_chunk.len());
                    file_sender.write_all(&bytes_chunk).await?;
                }
                Err(e) => {
                    bail!("LFS send error: {}", e);
                }
            }
        }
        file_sender.finish()?;

        //wait finish msg
        match file_connection.accept_bi().await {
            Ok((_file_sender, mut file_receiver)) => {
                file_receiver.read_to_end(1024).await?;
            }
            Err(_) => {
                info!("LFS connection closed.");
            }
        };
        info!(
            "Send lfs data to[{}], oid: {} successfully",
            to_peer_id, oid
        );
        Ok(())
    }

    async fn receive_quic_msg_task(self: Arc<Self>, tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
        loop {
            let (_, mut quic_recv) = self.get_connection().accept_bi().await?;
            let buffer = quic_recv.read_to_end(1024 * 1024).await?;
            info!("QUIC Received:\n{}", String::from_utf8_lossy(&buffer));
            if tx.send(buffer).await.is_err() {
                info!("Receiver closed");
            }
        }
    }

    async fn handle_quic_msg_task(self: Arc<Self>, message: Vec<u8>) -> Result<()> {
        let data: ResponseData = serde_json::from_slice(&message)?;
        match data.func.as_str() {
            "request_git_clone" => {
                let path = String::from_utf8(data.data)?;
                self.response_git_clone(path, data.from).await?;
            }
            "request_lfs" => {
                let oid = String::from_utf8(data.data)?;
                self.response_lfs(oid, data.from).await?;
            }
            "nostr" => {
                receive_nostr(data.data).await?;
            }
            "" => {
                if let Some(sender) = self.req_senders.get(data.req_id.as_str()) {
                    let mut guard = sender.lock().await;
                    if let Some(tx) = guard.take() {
                        tx.send(data.data).expect("Sender error");
                    }
                }
            }
            _ => {
                error!("Unsupported function");
            }
        }
        Ok(())
    }

    async fn run_ping_task(self: Arc<Self>, peer_id: Arc<str>) -> Result<()> {
        loop {
            let (mut quic_send, _) = self.get_connection().open_bi().await?;

            let ping = RequestData {
                from: String::from(peer_id.as_ref()),
                data: vec![],
                func: "".to_string(),
                action: Action::Ping,
                to: "relay".to_string(),
                req_id: Uuid::new_v4().into(),
            };
            let json = serde_json::to_string(&ping)?;
            quic_send.write_all(json.as_ref()).await?;
            quic_send.finish()?;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    async fn send_request(self: Arc<Self>, data: RequestData) -> Result<()> {
        let connection = self.get_connection();
        let (mut sender, _) = connection.open_bi().await?;
        let json = serde_json::to_string(&data)?;
        sender.write_all(json.as_bytes()).await?;
        sender.finish()?;
        Ok(())
    }

    async fn send_nostr_msg(&self, client_message: ClientMessage) -> Result<RelayMessage> {
        let peer_id = self.get_peer_id();
        let client = self.wrapped_client();
        let req_id: String = Uuid::new_v4().into();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.req_senders
            .insert(req_id.clone(), Arc::new(Mutex::new(Some(tx))));
        tokio::spawn(async move {
            let send = RequestData {
                from: peer_id,
                data: client_message.as_json().as_bytes().to_vec(),
                func: "".to_string(),
                action: Action::Nostr,
                to: "".to_string(),
                req_id: req_id.clone(),
            };
            if let Err(e) = client.send_request(send).await {
                error!("failed to send nostr msg: {e}");
            }
        });
        let data = rx.await?;
        let data = RelayMessage::from_json(data)?;
        Ok(data)
    }
}

async fn receive_nostr(data: Vec<u8>) -> Result<()> {
    info!("Nostr data:{}", String::from_utf8(data.clone())?);
    let _relay_message: RelayMessage = serde_json::from_slice(data.as_slice())?;
    Ok(())
}

async fn wait_rx_with_timeout(rx: tokio::sync::oneshot::Receiver<Vec<u8>>) -> Result<Vec<u8>> {
    match time::timeout(Duration::from_secs(5), rx).await {
        Ok(r) => Ok(r.clone()?),
        Err(_) => {
            bail!("Timed out waiting for quic result");
        }
    }
}
