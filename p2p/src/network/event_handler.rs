use super::behaviour;
use crate::network::behaviour::{GitUploadPackReq, GitUploadPackRes};
use crate::node::ClientParas;
use crate::{get_pack_protocol, get_repo_full_path};
use bytes::Bytes;
use common::utils;
use git::protocol::{CommandType, RefCommand};
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{
    AddProviderOk, GetClosestPeersOk, GetProvidersOk, GetRecordOk, Kademlia, KademliaEvent,
    PeerRecord, PutRecordOk, QueryResult,
};
use libp2p::{identify, multiaddr, rendezvous, request_response, Swarm};
use std::path::Path;

pub const NAMESPACE: &str = "rendezvous_mega";

pub fn kad_event_handler(event: KademliaEvent) {
    if let KademliaEvent::OutboundQueryProgressed { result, .. } = event {
        match result {
            QueryResult::GetRecord(Ok(GetRecordOk::FoundRecord(PeerRecord { record, peer }))) => {
                let peer_id = match peer {
                    Some(id) => id.to_string(),
                    None => "local".to_string(),
                };
                tracing::info!(
                    "Got record key[{}]={},from {}",
                    String::from_utf8(record.key.to_vec()).unwrap(),
                    String::from_utf8(record.value).unwrap(),
                    peer_id
                );
            }
            QueryResult::GetRecord(Err(err)) => {
                tracing::error!("Failed to get record: {err:?}");
            }
            QueryResult::PutRecord(Ok(PutRecordOk { key })) => {
                tracing::info!(
                    "Successfully put record {:?}",
                    std::str::from_utf8(key.as_ref()).unwrap()
                );
            }
            QueryResult::PutRecord(Err(err)) => {
                tracing::error!("Failed to put record: {err:?}");
            }
            QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { peers, .. })) => {
                for x in peers {
                    tracing::info!("{}", x);
                }
            }
            QueryResult::GetClosestPeers(Err(err)) => {
                tracing::error!("Failed to get closest peers: {err:?}");
            }
            QueryResult::GetProviders(Ok(GetProvidersOk::FoundProviders { providers, .. }), ..) => {
                tracing::info!("FoundProviders: {providers:?}");
            }
            QueryResult::GetProviders(Err(e)) => {
                tracing::error!("GetProviders error: {e:?}");
            }
            QueryResult::StartProviding(Ok(AddProviderOk { key, .. }), ..) => {
                tracing::info!("StartProviding: {key:?}");
            }
            _ => {}
        }
    }
}

pub fn rendezvous_client_event_handler(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    event: rendezvous::client::Event,
) {
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
            if let Some(rendezvous_point) = client_paras.rendezvous_point {
                swarm.behaviour_mut().rendezvous.discover(
                    Some(rendezvous::Namespace::from_static(NAMESPACE)),
                    client_paras.cookie.clone(),
                    None,
                    rendezvous_point,
                )
            }
        }
        rendezvous::client::Event::Discovered {
            registrations,
            cookie: new_cookie,
            ..
        } => {
            client_paras.cookie.replace(new_cookie);
            for registration in registrations {
                for address in registration.record.addresses() {
                    let peer = registration.record.peer_id();
                    tracing::info!("Rendezvous Discovered peer {} at {}", peer, address);
                    if peer == *swarm.local_peer_id() {
                        continue;
                    }
                    //dial via relay
                    if let Some(bootstrap_address) = client_paras.bootstrap_node_addr.clone() {
                        swarm
                            .dial(
                                bootstrap_address
                                    .with(multiaddr::Protocol::P2pCircuit)
                                    .with(multiaddr::Protocol::P2p(peer)),
                            )
                            .unwrap();
                    }
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

pub fn rendezvous_server_event_handler(event: rendezvous::server::Event) {
    match event {
        rendezvous::server::Event::PeerRegistered { peer, registration } => {
            tracing::info!(
                "Peer {} registered for namespace '{}'",
                peer,
                registration.namespace
            );
        }
        rendezvous::server::Event::DiscoverServed {
            enquirer,
            registrations,
            ..
        } => {
            tracing::info!(
                "Served peer {} with {} registrations",
                enquirer,
                registrations.len()
            );
        }

        event => {
            tracing::info!("Rendezvous server event:{:?}", event);
        }
    }
}

pub fn identify_event_handler(kademlia: &mut Kademlia<MemoryStore>, event: identify::Event) {
    match event {
        identify::Event::Received { peer_id, info } => {
            tracing::info!("IdentifyEvent Received peer_id:{:?}", peer_id);
            tracing::info!("IdentifyEvent Received info:{:?}", info);
            for addr in info.listen_addrs {
                kademlia.add_address(&peer_id, addr);
            }
        }

        identify::Event::Error { error, .. } => {
            tracing::debug!("IdentifyEvent Error :{:?}", error);
        }
        _ => {}
    }
}

pub async fn git_upload_pack_event_handler(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    event: request_response::Event<GitUploadPackReq, GitUploadPackRes>,
) {
    match event {
        request_response::Event::Message { message, .. } => match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                //receive git upload pack request
                tracing::debug!(
                    "Git upload pack event handler, {:?}, {:?}",
                    request,
                    channel
                );
                let commend = request.0;
                // command pull:
                // git-upload-pack /root/repotest/src.git
                let command_vec: Vec<_> = commend.split_whitespace().collect();
                if command_vec.len() < 2 {
                    tracing::error!("Invalid command:{}", commend);
                    return;
                }
                let path = command_vec[1];
                tracing::info!("path: {}", path);
                match git_upload_pack_handler(path, client_paras).await {
                    Ok((send_pack_data, object_id)) => {
                        let _ = swarm
                            .behaviour_mut()
                            .git_upload_pack
                            .send_response(channel, GitUploadPackRes(send_pack_data, object_id));
                    }
                    Err(e) => {
                        tracing::error!("{}", e);
                        let response = format!("ERR: {}", e);
                        let _ = swarm.behaviour_mut().git_upload_pack.send_response(
                            channel,
                            GitUploadPackRes(response.into_bytes(), utils::ZERO_ID.to_string()),
                        );
                    }
                }
            }
            request_response::Message::Response {
                request_id,
                response,
            } => {
                // receive a git_upload_pack response
                tracing::info!(
                    "Git upload pack event response, request_id: {:?}",
                    request_id,
                );
                if let Some(repo_name) = client_paras.pending_git_upload_package.get(&request_id) {
                    let package_data = response.0;
                    let object_id = response.1;
                    if package_data.starts_with("ERR:".as_bytes()) {
                        tracing::error!("{}", String::from_utf8(package_data).unwrap());
                        return;
                    }
                    let path = get_repo_full_path(repo_name);
                    let mut pack_protocol =
                        get_pack_protocol(&path, client_paras.storage.clone()).await;
                    let command = RefCommand {
                        ref_name: String::from("refs/heads/master"),
                        old_id: String::from("0000000000000000000000000000000000000000"),
                        new_id: object_id,
                        status: String::from("ok"),
                        error_msg: String::new(),
                        command_type: CommandType::Create,
                    };
                    pack_protocol.command_list.push(command);
                    // let result = command
                    //     .unpack(client_paras.storage.clone(), &mut Bytes::from(package_data))
                    //     .await;
                    let result = pack_protocol
                        .git_receive_pack(Bytes::from(package_data))
                        .await;
                    match result {
                        Ok(_) => {
                            tracing::info!("Save git package successfully :{}", repo_name);
                        }
                        Err(e) => {
                            tracing::error!("{}", e);
                        }
                    }
                }
            }
        },
        request_response::Event::OutboundFailure { peer, error, .. } => {
            tracing::error!("Git upload pack outbound failure: {:?},{:?}", peer, error);
        }
        event => {
            tracing::debug!("Request_response event:{:?}", event);
        }
    }
}

//After accepting the event, a new event needs to be triggered
pub fn kad_event_callback(
    _swarm: &mut Swarm<behaviour::Behaviour>,
    _client_paras: &mut ClientParas,
    event: KademliaEvent,
) {
    match event {
        KademliaEvent::OutboundQueryProgressed {
            result: QueryResult::GetProviders(Ok(GetProvidersOk::FoundProviders { .. })),
            ..
        } => {
            //check if there is pending_get_file
            // if let Some(file_name) = client_paras.pending_request_file.get(&id) {
            //     if providers.is_empty() {
            //         tracing::error!("Cannot find file {} from p2p network", file_name,);
            //     }
            //     //Trigger file download request
            //     for peer_id in providers {
            //         let request_file_id = swarm
            //             .behaviour_mut()
            //             .request_response
            //             .send_request(&peer_id, FileRequest(file_name.to_string()));
            //         tracing::info!("Get File {} from peer {}", file_name, peer_id);
            //         client_paras
            //             .pending_get_file
            //             .insert(request_file_id, file_name.clone());
            //     }
            // }
            // client_paras.pending_request_file.remove(&id);
            println!("111");
        }
        _event => {}
    }
}

async fn git_upload_pack_handler(
    path: &str,
    client_paras: &mut ClientParas,
) -> Result<(Vec<u8>, String), String> {
    let pack_protocol = get_pack_protocol(path, client_paras.storage.clone()).await;
    let object_id = pack_protocol.get_head_object_id(Path::new(path)).await;
    if object_id == *utils::ZERO_ID {
        return Err("Repository not found".to_string());
    }
    tracing::info!("object_id:{}", object_id);
    let send_pack_data = match pack_protocol.get_full_pack_data(Path::new(path)).await {
        Ok(send_pack_data) => send_pack_data,
        Err(e) => {
            tracing::error!("{}", e);
            return Err(e.to_string());
        }
    };
    // tracing::info!("send_pack_data:{:?}", send_pack_data);
    Ok((send_pack_data, object_id))
}
