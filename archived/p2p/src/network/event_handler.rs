use std::{collections::HashMap, path::Path, sync::Arc};

use common::utils;
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use libp2p::kad::{GetRecordResult, PeerRecord};
use libp2p::multiaddr::Protocol;
use libp2p::rendezvous::Cookie;
use libp2p::request_response::OutboundFailure;
use libp2p::{kad, multiaddr, rendezvous, request_response};
use libp2p::{
    kad::{GetRecordError, GetRecordOk, PutRecordError, PutRecordOk, Quorum},
    request_response::OutboundRequestId,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use storage::driver::database::storage::ObjectStorage;

use crate::network::{get_all_git_obj_ids, git_upload_pack_handler};
use crate::{get_pack_protocol, nostr::NostrRes};

use behaviour::{Behaviour, Event, GitObjectRes};

use super::behaviour::{GitInfoRefsRes, GitUploadPackRes};
use super::{behaviour, Command};

pub const NAMESPACE: &str = "rendezvous_mega";

pub(crate) struct EventLoop {
    swarm: Swarm<Behaviour>,
    storage: Arc<dyn ObjectStorage>,
    command_receiver: mpsc::Receiver<Command>,
    cookie: Option<Cookie>,
    bootstrap_node_addr: String,
    relay_peer_id: String,
    pending_dial: HashMap<Multiaddr, oneshot::Sender<PeerId>>,
    pending_put_record: HashMap<kad::QueryId, oneshot::Sender<Result<PutRecordOk, PutRecordError>>>,
    pending_get_record: HashMap<kad::QueryId, oneshot::Sender<Result<GetRecordOk, GetRecordError>>>,
    pending_git_upload_pack:
        HashMap<OutboundRequestId, oneshot::Sender<Result<GitUploadPackRes, OutboundFailure>>>,
    pending_git_info_refs:
        HashMap<OutboundRequestId, oneshot::Sender<Result<GitInfoRefsRes, OutboundFailure>>>,
    pending_git_object:
        HashMap<OutboundRequestId, oneshot::Sender<Result<GitObjectRes, OutboundFailure>>>,
    pending_nostr: HashMap<OutboundRequestId, oneshot::Sender<Result<NostrRes, OutboundFailure>>>,
}

impl EventLoop {
    pub fn new(
        swarm: Swarm<Behaviour>,
        storage: Arc<dyn ObjectStorage>,
        command_receiver: mpsc::Receiver<Command>,
    ) -> Self {
        Self {
            swarm,
            storage,
            command_receiver,
            cookie: None,
            bootstrap_node_addr: String::new(),
            relay_peer_id: String::new(),
            pending_dial: Default::default(),
            pending_put_record: Default::default(),
            pending_get_record: Default::default(),
            pending_git_upload_pack: Default::default(),
            pending_git_info_refs: Default::default(),
            pending_git_object: Default::default(),
            pending_nostr: Default::default(),
        }
    }

    pub async fn run(mut self) {
        // let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();
        loop {
            futures::select! {
                event = self.swarm.next() => self.handle_event(event.expect("Swarm stream to be infinite.")).await  ,
                command = self.command_receiver.next() => match command {
                    Some(c) => self.handle_command(c).await,
                    // Command channel closed, thus shutting down the network event loop.
                    None=>  return,
                }
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<Event>) {
        match event {
            SwarmEvent::Behaviour(Event::Kademlia(kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::StartProviding(add_provider_result),
                ..
            })) => {
                println!("Kademlia StartProviding:{:?}", add_provider_result);
            }
            SwarmEvent::Behaviour(Event::Kademlia(kad::Event::OutboundQueryProgressed {
                result:
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                        providers,
                        ..
                    })),
                ..
            })) => {
                println!("Kademlia GetProviders:{:?}", providers);
            }
            SwarmEvent::Behaviour(Event::Kademlia(kad::Event::OutboundQueryProgressed {
                id,
                result: kad::QueryResult::GetRecord(get_record_result),
                ..
            })) => {
                if let GetRecordResult::Ok(GetRecordOk::FoundRecord(PeerRecord {
                    peer,
                    record,
                    ..
                })) = get_record_result.clone()
                {
                    if let Ok(v) = String::from_utf8(record.value) {
                        tracing::info!(
                            "Kademlia GetRecordOk, peer:{:?},key:{:?},value:{}",
                            peer,
                            record.key,
                            v
                        );
                    }
                }

                if let Some(sender) = self.pending_get_record.remove(&id) {
                    sender
                        .send(get_record_result.clone())
                        .expect("Receiver not to be dropped");
                }
            }
            SwarmEvent::Behaviour(Event::Kademlia(kad::Event::OutboundQueryProgressed {
                result:
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { closest_peers },
                    )),
                ..
            })) => {
                println!(
                    "Kademlia GetProviders FinishedWithNoAdditionalRecord :{:?}",
                    closest_peers
                );
            }
            SwarmEvent::Behaviour(Event::Kademlia(kad::Event::OutboundQueryProgressed {
                id,
                result: kad::QueryResult::PutRecord(put_record_result),
                ..
            })) => {
                if let Some(sender) = self.pending_put_record.remove(&id) {
                    sender
                        .send(put_record_result.clone())
                        .expect("Receiver not to be dropped");
                }
                println!("Kademlia PutRecordResult:{:?}", put_record_result);
            }
            SwarmEvent::Behaviour(Event::Kademlia(e)) => {
                println!("Kademlia:{:?}", e);
            }
            SwarmEvent::IncomingConnection { .. } => {}
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, endpoint.get_remote_address().clone());
                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(endpoint.get_remote_address()) {
                        let _ = sender.send(peer_id);
                    }
                }
            }
            SwarmEvent::ConnectionClosed { .. } => {}
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                println!("OutgoingConnectionError:{:?},{:?}", peer_id, error);
            }
            SwarmEvent::IncomingConnectionError { .. } => {}
            SwarmEvent::Dialing {
                peer_id: Some(peer_id),
                ..
            } => println!("Dialing {peer_id}"),
            SwarmEvent::NewListenAddr {
                listener_id: _,
                address,
            } => {
                self.swarm.add_external_address(address);
            }
            //rendezvous
            SwarmEvent::Behaviour(Event::Rendezvous(event)) => {
                match event {
                    rendezvous::client::Event::Registered {
                        namespace,
                        ttl,
                        rendezvous_node,
                    } => {
                        tracing::info!(
                            "Registered for namespace '{}' at rendezvous point {} for the next {} seconds",
                            namespace,
                            rendezvous_node,
                            ttl
                        );
                        self.swarm.behaviour_mut().rendezvous.discover(
                            Some(rendezvous::Namespace::from_static(NAMESPACE)),
                            self.cookie.clone(),
                            None,
                            rendezvous_node,
                        );
                    }
                    rendezvous::client::Event::Discovered {
                        registrations,
                        cookie: new_cookie,
                        ..
                    } => {
                        self.cookie.replace(new_cookie);
                        for registration in registrations {
                            for address in registration.record.addresses() {
                                let peer = registration.record.peer_id();
                                tracing::info!(
                                    "Rendezvous Discovered peer {} at {}",
                                    peer,
                                    address
                                );
                                if peer == *self.swarm.local_peer_id() {
                                    continue;
                                }
                                let p2p_suffix = Protocol::P2p(peer);
                                let address_with_p2p = if !address
                                    .ends_with(&Multiaddr::empty().with(p2p_suffix.clone()))
                                {
                                    address.clone().with(p2p_suffix)
                                } else {
                                    address.clone()
                                };

                                self.swarm.dial(address_with_p2p).unwrap();
                                //dial via relay
                                // let bootstrap_node_addr: Multiaddr =
                                //     self.bootstrap_node.parse().unwrap();
                                // self.swarm
                                //     .dial(
                                //         bootstrap_node_addr
                                //             .with(multiaddr::Protocol::P2pCircuit)
                                //             .with(multiaddr::Protocol::P2p(peer)),
                                //     )
                                //     .unwrap();
                            }
                        }
                    }
                    rendezvous::client::Event::RegisterFailed { error, .. } => {
                        tracing::error!("Failed to register:  error_code={:?}", error);
                    }
                    rendezvous::client::Event::DiscoverFailed {
                        rendezvous_node,
                        namespace,
                        error,
                    } => {
                        tracing::error!(
                            "Failed to discover: rendezvous_node={}, namespace={:?}, error_code={:?}",
                            rendezvous_node,
                            namespace,
                            error
                        );
                    }
                    event => {
                        tracing::debug!("Rendezvous client event:{:?}", event);
                    }
                }
            }
            //GitUploadPack events
            SwarmEvent::Behaviour(Event::GitUploadPack(request_response::Event::Message {
                message,
                ..
            })) => {
                match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        //receive git upload pack request
                        tracing::debug!(
                            "Git upload pack event handler, {:?}, {:?}",
                            request,
                            channel
                        );
                        let want = request.0;
                        let have = request.1;
                        let path = request.2;
                        tracing::info!("path: {}", path);
                        match git_upload_pack_handler(&path, self.storage.clone(), want, have).await
                        {
                            Ok((send_pack_data, object_id)) => {
                                let _ = self.swarm.behaviour_mut().git_upload_pack.send_response(
                                    channel,
                                    GitUploadPackRes(send_pack_data, object_id),
                                );
                            }
                            Err(e) => {
                                tracing::error!("{}", e);
                                let response = format!("ERR: {}", e);
                                let _ = self.swarm.behaviour_mut().git_upload_pack.send_response(
                                    channel,
                                    GitUploadPackRes(
                                        response.into_bytes(),
                                        utils::ZERO_ID.to_string(),
                                    ),
                                );
                            }
                        }
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        if let Some(sender) = self.pending_git_upload_pack.remove(&request_id) {
                            sender
                                .send(Ok(response))
                                .expect("Receiver not to be dropped");
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(Event::GitUploadPack(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            )) => {
                tracing::error!("GitUploadPack OutboundFailure:\n {} \nfrom {}", error, peer);
                if let Some(sender) = self.pending_git_upload_pack.remove(&request_id) {
                    sender.send(Err(error)).expect("Receiver not to be dropped");
                }
            }
            //GitInfoRefs events
            SwarmEvent::Behaviour(Event::GitInfoRefs(request_response::Event::Message {
                message,
                ..
            })) => {
                match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        //receive git info refs  request
                        tracing::debug!(
                            "Receive git info refs event, {:?}, {:?}",
                            request,
                            channel
                        );
                        let path = request.0;
                        tracing::info!("path: {}", path);
                        let git_ids_they_have = request.1;
                        tracing::info!("git_ids_they_have: {:?}", git_ids_they_have);
                        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
                        let ref_git_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
                        let mut git_obj_ids =
                            get_all_git_obj_ids(&path, self.storage.clone()).await;
                        if !git_ids_they_have.is_empty() {
                            git_obj_ids.retain(|id| !git_ids_they_have.contains(id));
                        }
                        tracing::info!("git_ids_they_need: {:?}", git_obj_ids);
                        let _ = self
                            .swarm
                            .behaviour_mut()
                            .git_info_refs
                            .send_response(channel, GitInfoRefsRes(ref_git_id, git_obj_ids));
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        if let Some(sender) = self.pending_git_info_refs.remove(&request_id) {
                            sender
                                .send(Ok(response))
                                .expect("Receiver not to be dropped");
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(Event::GitInfoRefs(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            )) => {
                tracing::error!("GitInfoRefs OutboundFailure:\n {} \nfrom {}", error, peer);
                if let Some(sender) = self.pending_git_info_refs.remove(&request_id) {
                    sender.send(Err(error)).expect("Receiver not to be dropped");
                }
            }
            //GitInfoRefs events
            SwarmEvent::Behaviour(Event::GitObject(request_response::Event::Message {
                message,
                ..
            })) => {
                match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        //receive git object  request
                        tracing::debug!("Receive git object event, {:?}, {:?}", request, channel);
                        let path = request.0;
                        let git_ids = request.1;
                        tracing::info!("path: {}", path);
                        tracing::info!("git_ids: {:?}", git_ids);
                        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
                        let git_obj_models =
                            match pack_protocol.storage.get_obj_data_by_ids(git_ids).await {
                                Ok(models) => models,
                                Err(e) => {
                                    tracing::error!("{:?}", e);
                                    return;
                                }
                            };
                        let _ = self
                            .swarm
                            .behaviour_mut()
                            .git_object
                            .send_response(channel, GitObjectRes(git_obj_models));
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        if let Some(sender) = self.pending_git_object.remove(&request_id) {
                            sender
                                .send(Ok(response))
                                .expect("Receiver not to be dropped");
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(Event::GitObject(request_response::Event::OutboundFailure {
                peer,
                request_id,
                error,
            })) => {
                tracing::error!("GitObject OutboundFailure:\n {} \nfrom {}", error, peer);
                if let Some(sender) = self.pending_git_object.remove(&request_id) {
                    sender.send(Err(error)).expect("Receiver not to be dropped");
                }
            }
            //Nostr events
            SwarmEvent::Behaviour(Event::Nostr(request_response::Event::Message {
                message,
                peer,
                ..
            })) => match message {
                request_response::Message::Request { request, .. } => {
                    tracing::info!(
                        "Nostr client receive request:\n {} \nfrom {}",
                        request.0,
                        peer
                    );
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    tracing::info!(
                        "Nostr client receive response, \n {} \nfrom {}",
                        response.0,
                        peer
                    );
                    if let Some(sender) = self.pending_nostr.remove(&request_id) {
                        sender
                            .send(Ok(response))
                            .expect("Receiver not to be dropped");
                    }
                }
            },
            SwarmEvent::Behaviour(Event::Nostr(request_response::Event::OutboundFailure {
                peer,
                request_id,
                error,
            })) => {
                tracing::error!("Nostr OutboundFailure:\n {} \nfrom {}", error, peer);
                if let Some(sender) = self.pending_nostr.remove(&request_id) {
                    sender.send(Err(error)).expect("Receiver not to be dropped");
                }
            }

            e => println!("{e:?}"),
        }
    }

    async fn handle_command(&mut self, command: Command) {
        match command {
            Command::StartListening { addr, sender } => {
                let _ = match self.swarm.listen_on(addr.clone()) {
                    Ok(_) => sender.send(Ok(())),
                    Err(e) => sender.send(Err(Box::new(e))),
                };
            }
            Command::Dial { peer_addr, sender } => {
                match self.swarm.dial(peer_addr.clone()) {
                    Ok(()) => {
                        self.pending_dial.insert(peer_addr, sender);
                    }
                    Err(_e) => {}
                };
            }
            Command::RendezvousRegister {
                relay_peer_id,
                bootstrap_node_addr,
                sender,
            } => {
                self.bootstrap_node_addr = bootstrap_node_addr.clone().to_string();
                self.relay_peer_id = relay_peer_id.clone().to_string();
                let public_addr = bootstrap_node_addr
                    .clone()
                    .with(multiaddr::Protocol::P2pCircuit);

                // self.swarm.listen_on(public_addr.clone()).unwrap();
                self.swarm.add_external_address(public_addr);
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&relay_peer_id.clone(), bootstrap_node_addr.clone());
                let _ = match self.swarm.behaviour_mut().rendezvous.register(
                    rendezvous::Namespace::from_static(NAMESPACE),
                    relay_peer_id,
                    None,
                ) {
                    Ok(_) => sender.send(Ok(())),
                    Err(e) => sender.send(Err(Box::new(e))),
                };
            }
            Command::PutRecord { record, sender } => {
                if let Ok(query_id) = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record.clone(), Quorum::One)
                {
                    self.pending_put_record.insert(query_id, sender);
                } else {
                    eprintln!("Failed to store record:{:?}", record);
                }
            }
            Command::GetRecord { key, sender } => {
                let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);
                self.pending_get_record.insert(query_id, sender);
            }
            Command::GitUploadPack {
                peer_id,
                git_upload_pack_req,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .git_upload_pack
                    .send_request(&peer_id, git_upload_pack_req);
                self.pending_git_upload_pack.insert(request_id, sender);
            }
            Command::GitInfoRefs {
                peer_id,
                git_info_refs_req,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .git_info_refs
                    .send_request(&peer_id, git_info_refs_req);
                self.pending_git_info_refs.insert(request_id, sender);
            }
            Command::GitObject {
                peer_id,
                git_object_req,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .git_object
                    .send_request(&peer_id, git_object_req);
                self.pending_git_object.insert(request_id, sender);
            }
            Command::Nostr {
                peer_id,
                nostr_req,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .nostr
                    .send_request(&peer_id, nostr_req);
                self.pending_nostr.insert(request_id, sender);
            }
        }
    }
}
