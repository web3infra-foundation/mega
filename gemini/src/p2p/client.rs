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
use ceres::protocol::repo::Repo;
use common::utils::generate_id;
use dashmap::DashMap;
use futures_util::{StreamExt, TryStreamExt};
use jupiter::context::Context;
use lazy_static::lazy_static;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tag::Tag;
use mercury::internal::object::tree::Tree;
use mercury::internal::object::types::ObjectType;
use mercury::internal::pack::encode::PackEncoder;
use mercury::internal::pack::entry::Entry;
use mercury::internal::pack::Pack;
use quinn::crypto::rustls::QuicClientConfig;
use quinn::rustls::pki_types::pem::PemObject;
use quinn::rustls::pki_types::CertificateDer;
use quinn::rustls::pki_types::PrivateKeyDer;
use quinn::Connection;
use quinn::{rustls, ClientConfig, Endpoint};
use std::result::Result::Ok;
use tokio::join;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use tokio::sync::{mpsc, Mutex};
use tokio_util::io::ReaderStream;
use tracing::error;
use tracing::info;
use uuid::Uuid;
use vault::get_peerid;

use super::{LFSHeader, ALPN_QUIC_HTTP};
use crate::nostr::client_message::{ClientMessage, Filter, SubscriptionId};
use crate::nostr::event::NostrEvent;
use crate::nostr::relay_message::RelayMessage;
use crate::nostr::GitEvent;
use crate::p2p::RequestData;
use crate::p2p::ResponseData;
use crate::p2p::{Action, GitCloneHeader};
use crate::util::{
    get_git_model_by_path, get_repo_path, parse_pointer_data, repo_path_to_identifier,
};
use crate::{ca, RepoInfo};

struct MsgSingletonConnection {
    conn: Arc<Connection>,
}
static INSTANCE: OnceLock<MsgSingletonConnection> = OnceLock::new();

lazy_static! {
    //oneshot sender map
    static ref REQ_SENDER_MAP: DashMap<String, Arc<Mutex<Option<Sender<Vec<u8>>>>>> =
        DashMap::new();
}

impl MsgSingletonConnection {
    fn new(conn: Arc<Connection>) -> Self {
        MsgSingletonConnection { conn }
    }

    pub fn init(conn: Arc<Connection>) {
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

pub async fn run(context: Context, bootstrap_node: String) -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let connection = get_client_connection(bootstrap_node.clone()).await?;

    let connection = Arc::new(connection);
    MsgSingletonConnection::init(connection.clone());

    let (tx, mut rx) = mpsc::channel(8);

    let peer_id = get_peerid().await;

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
        let bootstrap_node = bootstrap_node.clone();
        let context = context.clone();
        tokio::spawn(async move {
            let data: ResponseData = match serde_json::from_slice(&message) {
                Ok(data) => data,
                Err(e) => {
                    error!("QUIC Received Error:\n{:?}", e);
                    return;
                }
            };
            match data.func.as_str() {
                "request_git_clone" => {
                    let path = String::from_utf8(data.data).unwrap();
                    response_git_clone(context.clone(), bootstrap_node.clone(), path, data.from)
                        .await
                        .unwrap();
                }
                "request_lfs" => {
                    let oid = String::from_utf8(data.data).unwrap();
                    response_lfs(context.clone(), bootstrap_node.clone(), oid, data.from)
                        .await
                        .unwrap();
                }
                "nostr" => {
                    receive_nostr(data.data).await.unwrap();
                }
                "" => {
                    if let Some(sender) = REQ_SENDER_MAP.get(data.req_id.as_str()) {
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
        });
    }

    Ok(())
}

pub async fn get_client_connection(bootstrap_node: String) -> Result<Connection> {
    let (user_cert, user_key) = get_user_cert_from_ca(bootstrap_node.clone()).await?;
    let ca_cert = get_ca_cert_from_ca(bootstrap_node.clone()).await?;

    let mut roots = rustls::RootCertStore::empty();

    roots.add(ca_cert)?;

    let mut client_crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert([user_cert].to_vec(), user_key)?;
    client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    client_crypto.enable_early_data = true;
    let client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto)?));
    let mut endpoint = Endpoint::client(SocketAddr::from_str("[::]:0").unwrap())?;
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

pub async fn call(to_peer_id: String, func: String, data: Vec<u8>) -> Result<Vec<u8>> {
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
            action: Action::Call,
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
    let data: ResponseData = serde_json::from_slice(&message)?;
    Ok(data.data)
}

pub async fn send(to_peer_id: String, func: String, data: Vec<u8>) -> Result<()> {
    let connection = MsgSingletonConnection::get_connection();

    let connection_clone = connection.clone();
    let local_peer_id = get_peerid().await;
    let t = tokio::spawn(async move {
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
    let _ = join!(t);
    Ok(())
}

pub async fn repo_share(context: Context, path: String) -> Result<String> {
    let storage = context.services.git_db_storage.clone();

    let repo: git_repo::Model = match get_git_model_by_path(context.clone(), path.clone()).await {
        None => {
            bail!("Repo not found: {}", path);
        }
        Some(repo) => repo.into(),
    };
    let commit = storage.get_last_commit_by_repo_id(repo.id).await.unwrap();
    let commit = match commit {
        Some(commit) => commit,
        None => {
            bail!("Repo commit error");
        }
    };
    let mut repo_info: RepoInfo = repo.clone().into();
    let identifier = repo_path_to_identifier(repo.repo_path).await;
    repo_info.identifier = identifier.clone();
    repo_info.commit = commit.commit_id;
    repo_info.update_time = commit.created_at.and_utc().timestamp();
    repo_info.origin = get_peerid().await;

    let connection = MsgSingletonConnection::get_connection();

    let connection_clone = connection.clone();
    let local_peer_id = get_peerid().await;
    let req_id: String = Uuid::new_v4().into();

    let (tx, rx) = tokio::sync::oneshot::channel();
    REQ_SENDER_MAP.insert(req_id.clone(), Arc::new(Mutex::new(Some(tx))));
    tokio::spawn(async move {
        let (mut sender, _) = connection_clone.open_bi().await.unwrap();
        let send = RequestData {
            from: local_peer_id.clone(),
            data: repo_info.to_json().into_bytes(),
            func: "".to_string(),
            action: Action::RepoShare,
            to: "".to_string(),
            req_id: req_id.clone(),
        };
        let json = serde_json::to_string(&send).unwrap();
        sender.write_all(json.as_bytes()).await.unwrap();
        sender.finish().unwrap();
    });

    let _ = rx.await?;
    info!("Repo share success: {}", identifier);
    Ok(identifier)
}

pub async fn request_git_clone(
    context: Context,
    bootstrap_node: String,
    path: String,
    to_peer_id: String,
) -> Result<()> {
    let storage = context.services.git_db_storage.clone();
    let model = storage
        .find_git_repo_exact_match(path.as_str())
        .await
        .unwrap();
    if model.is_some() {
        bail!("Repo path already exists");
    }

    // Register file connection to relay
    let file_connection = get_client_connection(bootstrap_node.clone()).await?;
    let (mut file_sender, mut _file_receiver) = file_connection.open_bi().await?;
    let peer_id = get_peerid().await;
    file_sender
        .write_all(
            format!(
                "git-clone-{}-{}|{}",
                peer_id.clone(),
                to_peer_id,
                "REQUEST_GIT_CLONE"
            )
            .as_bytes(),
        )
        .await?;
    file_sender.finish()?;

    //send git clone request msg via msg connection
    let (mut msg_sender, _) = MsgSingletonConnection::get_connection().open_bi().await?;

    let send = RequestData {
        from: get_peerid().await,
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
    let len = file_receiver.read(&mut header_buf).await?.unwrap();
    let header = String::from_utf8_lossy(&header_buf[..len]);
    let header: GitCloneHeader = serde_json::from_str(&header)?;
    let (target_id, from, git_path) = (header.target, header.from, header.git_path);
    if target_id != peer_id {
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
    let (sender, mut receiver) = mpsc::channel(1024);
    let p = Pack::new(
        None,
        Some(1024 * 1024 * 1024 * 4),
        Some(context.config.pack.pack_decode_cache_path.clone()),
        context.config.pack.clean_cache_after_decode,
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
            let context = context.clone();
            let bootstrap_node = bootstrap_node.clone();
            let to_peer_id = to_peer_id.clone();
            let t = tokio::spawn(async move {
                //try to download lfs
                request_lfs(
                    context.clone(),
                    bootstrap_node.clone(),
                    oid.0.to_string(),
                    to_peer_id.clone(),
                )
                .await
                .unwrap();
            });
            task.push(t);
        }
    }
    futures::future::join_all(task).await;
    //Save to db
    storage.save_git_repo(repo.clone().into()).await.unwrap();
    storage.save_entry(repo.repo_id, entry_list).await.unwrap();
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
        storage.save_ref(repo.repo_id, r).await.unwrap();
    }

    info!(
        "Git clone from[{}] with path[{}] successfully",
        to_peer_id, git_path
    );

    Ok(())
}

pub async fn response_git_clone(
    context: Context,
    bootstrap_node: String,
    path: String,
    to_peer_id: String,
) -> Result<()> {
    let storage = context.services.git_db_storage.clone();

    let repo: Repo = match get_git_model_by_path(context.clone(), path.clone()).await {
        None => {
            bail!("Repo not found: {}", path);
        }
        Some(repo) => repo.into(),
    };
    // Register file connection to relay
    let file_connection = get_client_connection(bootstrap_node).await?;
    let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
    let peer_id = get_peerid().await;
    file_sender
        .write_all(
            format!(
                "git-clone-{}-{}|{}",
                peer_id.clone(),
                to_peer_id,
                "RESPONSE_GIT_CLONE"
            )
            .as_bytes(),
        )
        .await?;
    file_sender.finish()?;

    //send git clone header
    let refs = storage.get_ref(repo.repo_id).await.unwrap();
    let header = GitCloneHeader {
        from: peer_id.clone(),
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
    let mut receiver = get_encode_git_objects_by_repo(context, repo).await;
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

pub async fn request_lfs(
    context: Context,
    bootstrap_node: String,
    oid: String,
    to_peer_id: String,
) -> Result<()> {
    // Register file connection to relay
    let file_connection = get_client_connection(bootstrap_node).await?;
    let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
    let peer_id = get_peerid().await;
    file_sender
        .write_all(format!("lfs-{}-{}|{}", peer_id.clone(), to_peer_id, "REQUEST_LFS").as_bytes())
        .await?;
    file_sender.finish()?;

    //send request lfs request msg via msg connection
    let (mut msg_sender, _) = MsgSingletonConnection::get_connection().open_bi().await?;

    let send = RequestData {
        from: get_peerid().await,
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
    let len = file_receiver.read(&mut header_buf).await?.unwrap();
    let header = String::from_utf8_lossy(&header_buf[..len]);
    let header: LFSHeader = serde_json::from_str(&header)?;
    info!("LFS handle receive, {:?}", header);
    if header.target != peer_id {
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
    let config = context.config.lfs.clone();
    let meta_to = lfs_objects::Model {
        oid: header.oid,
        size: header.size,
        exist: true,
        splited: config.local.enable_split,
    };

    let res = context.lfs_stg().new_lfs_object(meta_to).await;
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

    let result = handler::lfs_upload_object(&context.clone(), &req_obj, data).await;

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

pub async fn response_lfs(
    context: Context,
    bootstrap_node: String,
    oid: String,
    to_peer_id: String,
) -> Result<()> {
    info!("oid:{}", oid.clone());
    let result = context
        .lfs_stg()
        .get_lfs_object(oid.as_str())
        .await
        .unwrap();

    let lfs_object = match result {
        None => {
            bail!("LFS not found: {}", oid);
        }
        Some(o) => o,
    };
    // Register lfs connection to relay
    let file_connection = get_client_connection(bootstrap_node).await?;
    let (mut file_sender, _file_receiver) = file_connection.open_bi().await?;
    let peer_id = get_peerid().await;
    file_sender
        .write_all(format!("lfs-{}-{}|{}", peer_id.clone(), to_peer_id, "RESPONSE_LFS").as_bytes())
        .await?;
    file_sender.finish()?;

    //send lfs header
    let header = LFSHeader {
        from: peer_id.clone(),
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
    let mut result = lfs_download_object(context.clone(), oid.clone())
        .await
        .unwrap();
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

pub async fn subscribe_repo(identifier: String) -> Result<()> {
    let filters = vec![Filter::new().repo_uri(identifier)];
    let subscription_id = get_peerid().await;
    let client_req = ClientMessage::new_req(SubscriptionId::new(subscription_id), filters);

    let relay_message = send_nostr_msg(client_req).await?;
    info!("Subscribe repo result: {}", relay_message.as_json());
    Ok(())
}

pub async fn send_git_event(git_event: GitEvent) -> Result<()> {
    let keypair = vault::get_keypair().await;
    let event = NostrEvent::new_git_event(keypair, git_event);
    let client_message = ClientMessage::new_event(event);
    let relay_message = send_nostr_msg(client_message).await?;
    info!("Sent git event result: {}", relay_message.as_json());
    Ok(())
}

async fn send_nostr_msg(client_message: ClientMessage) -> Result<RelayMessage> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let req_id: String = Uuid::new_v4().into();
    let local_peer_id = get_peerid().await;
    REQ_SENDER_MAP.insert(req_id.clone(), Arc::new(Mutex::new(Some(tx))));
    let connection = MsgSingletonConnection::get_connection();
    tokio::spawn(async move {
        let (mut sender, _) = connection.open_bi().await.unwrap();
        let send = RequestData {
            from: local_peer_id.clone(),
            data: client_message.as_json().as_bytes().to_vec(),
            func: "".to_string(),
            action: Action::Nostr,
            to: "".to_string(),
            req_id: req_id.clone(),
        };
        let json = serde_json::to_string(&send).unwrap();
        sender.write_all(json.as_bytes()).await.unwrap();
        sender.finish().unwrap();
    });
    let data = rx.await?;
    let data = RelayMessage::from_json(data)?;
    Ok(data)
}

pub async fn receive_nostr(data: Vec<u8>) -> Result<()> {
    let event = NostrEvent::from_json(data)?;
    info!("Receive nostr event:{:?}", event);
    Ok(())
}

async fn get_encode_git_objects_by_repo(context: Context, repo: Repo) -> Receiver<Vec<u8>> {
    let storage = context.services.git_db_storage.clone();
    let raw_storage = context.services.raw_db_storage.clone();
    let (entry_tx, entry_rx) = mpsc::channel(32);
    let (stream_tx, stream_rx) = mpsc::channel(32);
    let total = storage.get_obj_count_by_repo_id(repo.repo_id).await;
    let encoder = PackEncoder::new(total, 0, stream_tx);
    encoder.encode_async(entry_rx).await.unwrap();
    let repo_id = repo.repo_id;

    let mut commit_stream = storage.get_commits_by_repo_id(repo_id).await.unwrap();

    while let Some(model) = commit_stream.next().await {
        match model {
            Ok(m) => {
                let c: Commit = m.into();
                let entry = c.into();
                entry_tx.send(entry).await.unwrap();
            }
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }

    let mut tree_stream = storage.get_trees_by_repo_id(repo_id).await.unwrap();
    while let Some(model) = tree_stream.next().await {
        match model {
            Ok(m) => {
                let t: Tree = m.into();
                let entry = t.into();
                entry_tx.send(entry).await.unwrap();
            }
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }
    let mut bid_stream = storage.get_blobs_by_repo_id(repo_id).await.unwrap();
    let mut bids = vec![];
    while let Some(model) = bid_stream.next().await {
        match model {
            Ok(m) => bids.push(m.blob_id),
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }

    let mut blob_handler = vec![];
    for chunk in bids.chunks(10000) {
        let raw_storage = raw_storage.clone();
        let sender_clone = entry_tx.clone();
        let chunk_clone = chunk.to_vec();
        let handler = tokio::spawn(async move {
            let mut blob_stream = raw_storage.get_raw_blobs_stream(chunk_clone).await.unwrap();
            while let Some(model) = blob_stream.next().await {
                match model {
                    Ok(m) => {
                        let b: Blob = m.into();
                        let entry: Entry = b.into();
                        sender_clone.send(entry).await.unwrap();
                    }
                    Err(err) => eprintln!("Error: {:?}", err),
                }
            }
        });
        blob_handler.push(handler);
    }

    let tags = storage.get_tags_by_repo_id(repo_id).await.unwrap();
    for m in tags.into_iter() {
        let c: Tag = m.into();
        let entry: Entry = c.into();
        entry_tx.send(entry).await.unwrap();
    }
    drop(entry_tx);
    stream_rx
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
