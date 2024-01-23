//!
//!
//!
//!
//!
//!

pub mod behaviour;
pub mod event_handler;

use std::{collections::HashSet, error::Error, path::Path, sync::Arc, time::Duration};

use common::utils;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt as _,
};
use libp2p::{
    dcutr, identify,
    kad::{self, RecordKey},
    noise, rendezvous,
    request_response::{self, OutboundFailure},
    tcp, yamux,
};
use libp2p::{
    identity::Keypair,
    kad::{store::MemoryStore, GetRecordError, GetRecordOk, PutRecordError, PutRecordOk, Record},
    request_response::ProtocolSupport,
    Multiaddr, PeerId, StreamProtocol,
};
use storage::driver::database::storage::ObjectStorage;

use crate::{
    cbor, get_pack_protocol,
    nostr::{NostrReq, NostrRes},
};

use self::{
    behaviour::{
        Behaviour, GitInfoRefsReq, GitInfoRefsRes, GitObjectReq, GitObjectRes, GitUploadPackReq,
        GitUploadPackRes,
    },
    event_handler::EventLoop,
};

pub(crate) async fn new(
    local_key: Keypair,
    storage: Arc<dyn ObjectStorage>,
) -> Result<(Client, EventLoop), Box<dyn Error>> {
    let peer_id = local_key.public().to_peer_id();

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_async_std()
        .with_tcp(
            tcp::Config::default().port_reuse(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|keypair, client_behaviour| Behaviour {
            relay_client: client_behaviour,
            identify: identify::Behaviour::new(identify::Config::new(
                "/mega/0.0.1".to_string(),
                keypair.public(),
            )),
            dcutr: dcutr::Behaviour::new(peer_id),
            //DHT
            kademlia: kad::Behaviour::new(peer_id, MemoryStore::new(peer_id)),
            //discover
            rendezvous: rendezvous::client::Behaviour::new(keypair.clone()),
            // git pull, git clone
            git_upload_pack: cbor::Behaviour::new(
                [(
                    StreamProtocol::new("/mega/git_upload_pack"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default().with_request_timeout(Duration::from_secs(60)),
            ),
            // git info refs
            git_info_refs: cbor::Behaviour::new(
                [(
                    StreamProtocol::new("/mega/git_info_refs"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
            // git download git_obj
            git_object: cbor::Behaviour::new(
                [(StreamProtocol::new("/mega/git_obj"), ProtocolSupport::Full)],
                request_response::Config::default().with_request_timeout(Duration::from_secs(60)),
            ),
            nostr: cbor::Behaviour::new(
                [(StreamProtocol::new("/mega/nostr"), ProtocolSupport::Full)],
                request_response::Config::default(),
            ),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    swarm
        .behaviour_mut()
        .kademlia
        .set_mode(Some(kad::Mode::Server));

    let (command_sender, command_receiver) = mpsc::channel(64);

    Ok((
        Client {
            sender: command_sender,
        },
        EventLoop::new(swarm, storage, command_receiver),
    ))
}

#[derive(Clone)]
pub struct Client {
    sender: mpsc::Sender<Command>,
}

impl Client {
    pub(crate) async fn start_listening(
        &mut self,
        addr: Multiaddr,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::StartListening { addr, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn dial(&mut self, peer_addr: Multiaddr) -> PeerId {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Dial { peer_addr, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn rendezvous_register(
        &mut self,
        relay_peer_id: PeerId,
        bootstrap_node_addr: Multiaddr,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RendezvousRegister {
                relay_peer_id,
                bootstrap_node_addr,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn put_record(
        &mut self,
        record: Record,
    ) -> Result<PutRecordOk, PutRecordError> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::PutRecord { record, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn get_record(
        &mut self,
        key: RecordKey,
    ) -> Result<GetRecordOk, GetRecordError> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::GetRecord { key, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn git_upload_pack(
        &mut self,
        peer_id: PeerId,
        git_upload_pack_req: GitUploadPackReq,
    ) -> Result<GitUploadPackRes, OutboundFailure> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::GitUploadPack {
                peer_id,
                git_upload_pack_req,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn git_info_refs(
        &mut self,
        peer_id: PeerId,
        git_info_refs_req: GitInfoRefsReq,
    ) -> Result<GitInfoRefsRes, OutboundFailure> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::GitInfoRefs {
                peer_id,
                git_info_refs_req,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn git_object(
        &mut self,
        peer_id: PeerId,
        git_object_req: GitObjectReq,
    ) -> Result<GitObjectRes, OutboundFailure> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::GitObject {
                peer_id,
                git_object_req,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub(crate) async fn nostr(
        &mut self,
        peer_id: PeerId,
        nostr_req: NostrReq,
    ) -> Result<NostrRes, OutboundFailure> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Nostr {
                peer_id,
                nostr_req,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }
}

#[derive(Debug)]
pub enum Command {
    StartListening {
        addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    Dial {
        peer_addr: Multiaddr,
        sender: oneshot::Sender<PeerId>,
    },
    RendezvousRegister {
        relay_peer_id: PeerId,
        bootstrap_node_addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    PutRecord {
        record: Record,
        sender: oneshot::Sender<Result<PutRecordOk, PutRecordError>>,
    },
    GetRecord {
        key: RecordKey,
        sender: oneshot::Sender<Result<GetRecordOk, GetRecordError>>,
    },
    GitUploadPack {
        peer_id: PeerId,
        git_upload_pack_req: GitUploadPackReq,
        sender: oneshot::Sender<Result<GitUploadPackRes, OutboundFailure>>,
    },
    GitInfoRefs {
        peer_id: PeerId,
        git_info_refs_req: GitInfoRefsReq,
        sender: oneshot::Sender<Result<GitInfoRefsRes, OutboundFailure>>,
    },
    GitObject {
        peer_id: PeerId,
        git_object_req: GitObjectReq,
        sender: oneshot::Sender<Result<GitObjectRes, OutboundFailure>>,
    },
    Nostr {
        peer_id: PeerId,
        nostr_req: NostrReq,
        sender: oneshot::Sender<Result<NostrRes, OutboundFailure>>,
    },
}

async fn git_upload_pack_handler(
    path: &str,
    storage: Arc<dyn ObjectStorage>,
    want: HashSet<String>,
    have: HashSet<String>,
) -> Result<(Vec<u8>, String), String> {
    let pack_protocol = get_pack_protocol(path, storage.clone());
    let object_id = pack_protocol.get_head_object_id(Path::new(path)).await;
    if object_id == *utils::ZERO_ID {
        return Err("Repository not found".to_string());
    }
    tracing::info!("object_id:{}", object_id);
    tracing::info!("want: {:?}, have: {:?}", want, have);
    if have.is_empty() {
        //clone
        let send_pack_data = match pack_protocol.get_full_pack_data(Path::new(path)).await {
            Ok(send_pack_data) => send_pack_data,
            Err(e) => {
                tracing::error!("{}", e);
                return Err(e.to_string());
            }
        };
        Ok((send_pack_data, object_id))
    } else {
        //pull
        let send_pack_data = match pack_protocol
            .get_incremental_pack_data(Path::new(&path), &want, &have)
            .await
        {
            Ok(send_pack_data) => send_pack_data,
            Err(e) => {
                tracing::error!("{}", e);
                return Err(e.to_string());
            }
        };
        Ok((send_pack_data, object_id))
    }
}

pub async fn get_all_git_obj_ids(path: &str, storage: Arc<dyn ObjectStorage>) -> Vec<String> {
    let pack_protocol = get_pack_protocol(path, storage.clone());
    let mut git_ids: Vec<String> = Vec::new();
    if let Ok(commit_models) = pack_protocol.storage.get_all_commits_by_path(path).await {
        commit_models.iter().for_each(|model| {
            git_ids.push(model.git_id.clone());
        });
    }
    if let Ok(blob_and_tree) = pack_protocol
        .storage
        .get_node_by_path(Path::new(&path))
        .await
    {
        blob_and_tree.iter().for_each(|model| {
            git_ids.push(model.git_id.clone());
        });
    }
    git_ids
}

#[cfg(test)]
mod tests {}
