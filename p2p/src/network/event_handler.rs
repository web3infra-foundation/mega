use super::behaviour;
use crate::network::behaviour::{
    GitInfoRefsReq, GitInfoRefsRes, GitObjectReq, GitObjectRes, GitUploadPackReq, GitUploadPackRes,
};
use crate::node::ClientParas;
use crate::{get_pack_protocol, get_repo_full_path};
use bytes::Bytes;
use common::utils;
use entity::git_obj;
use git::protocol::{CommandType, RefCommand};
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{
    AddProviderOk, GetClosestPeersOk, GetProvidersOk, GetRecordOk, Kademlia, KademliaEvent,
    PeerRecord, PutRecordOk, QueryResult,
};
use libp2p::{identify, multiaddr, rendezvous, request_response, Swarm};
use sea_orm::Set;
use std::collections::HashSet;
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
                let want = request.0;
                let have = request.1;
                let path = request.2;
                tracing::info!("path: {}", path);
                match git_upload_pack_handler(&path, client_paras, want, have).await {
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
                    client_paras.pending_git_upload_package.remove(&request_id);
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

pub async fn git_info_refs_event_handler(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    event: request_response::Event<GitInfoRefsReq, GitInfoRefsRes>,
) {
    match event {
        request_response::Event::Message { message, peer, .. } => match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                //receive git info refs  request
                tracing::debug!("Receive git info refs event, {:?}, {:?}", request, channel);
                let path = request.0;
                tracing::info!("path: {}", path);
                let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
                let ref_git_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
                let mut git_ids: Vec<String> = Vec::new();
                if let Ok(commit_models) =
                    pack_protocol.storage.get_all_commits_by_path(&path).await
                {
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
                let _ = swarm
                    .behaviour_mut()
                    .git_info_refs
                    .send_response(channel, GitInfoRefsRes(ref_git_id, git_ids));
            }
            request_response::Message::Response {
                request_id,
                response,
            } => {
                //receive git info refs  response
                tracing::info!("Response git info refs event, request_id: {:?}", request_id);
                if let Some(repo_name) = client_paras.pending_git_pull.get(&request_id) {
                    //pull request
                    let ref_git_id = response.0;
                    let _git_ids = response.1;
                    tracing::info!("repo_name: {}", repo_name);
                    tracing::info!("ref_git_id: {:?}", ref_git_id);
                    if ref_git_id == *utils::ZERO_ID {
                        eprintln!("Repo not found");
                        return;
                    }
                    let path = get_repo_full_path(repo_name);
                    let pack_protocol =
                        get_pack_protocol(&path, client_paras.storage.clone()).await;
                    //generate want and have collection
                    let mut want: HashSet<String> = HashSet::new();
                    let mut have: HashSet<String> = HashSet::new();
                    want.insert(ref_git_id);
                    let commit_models = pack_protocol
                        .storage
                        .get_all_commits_by_path(&path)
                        .await
                        .unwrap();
                    commit_models.iter().for_each(|model| {
                        have.insert(model.git_id.clone());
                    });
                    //send new request to git_upload_pack
                    let new_request_id = swarm
                        .behaviour_mut()
                        .git_upload_pack
                        .send_request(&peer, GitUploadPackReq(want, have, path));
                    client_paras
                        .pending_git_upload_package
                        .insert(new_request_id, repo_name.to_string());
                    client_paras.pending_git_pull.remove(&request_id);
                    return;
                }
                if let Some(repo_name) = client_paras.pending_git_obj_download.get(&request_id) {
                    // git_obj_download request
                    let _ref_git_id = response.0;
                    let git_ids = response.1;
                    let path = get_repo_full_path(repo_name);
                    tracing::info!("path: {}", path);
                    tracing::info!("git_ids: {:?}", git_ids);
                    //trying to download git_obj from peer
                    let new_request_id = swarm
                        .behaviour_mut()
                        .git_object
                        .send_request(&peer, GitObjectReq(path, git_ids));
                    client_paras
                        .pending_git_obj_download
                        .insert(new_request_id, repo_name.to_string());
                }
            }
        },
        request_response::Event::OutboundFailure { peer, error, .. } => {
            tracing::error!("Git info refs outbound failure: {:?},{:?}", peer, error);
        }
        event => {
            tracing::debug!("Request_response event:{:?}", event);
        }
    }
}

pub async fn git_object_event_handler(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    event: request_response::Event<GitObjectReq, GitObjectRes>,
) {
    match event {
        request_response::Event::Message { message, .. } => match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                //receive git object  request
                tracing::debug!("Receive git object event, {:?}, {:?}", request, channel);
                let path = request.0;
                let git_ids = request.1;
                tracing::info!("path: {}", path);
                tracing::info!("git_ids: {:?}", git_ids);
                let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
                let git_obj_models = match pack_protocol.storage.get_obj_data_by_ids(git_ids).await
                {
                    Ok(models) => models,
                    Err(e) => {
                        tracing::error!("{:?}", e);
                        return;
                    }
                };
                let _ = swarm
                    .behaviour_mut()
                    .git_object
                    .send_response(channel, GitObjectRes(git_obj_models));
            }
            request_response::Message::Response {
                request_id,
                response,
            } => {
                //receive git object response
                tracing::info!("Response git object event, request_id: {:?}", request_id);
                let git_obj_models = response.0;
                tracing::info!("Receive {:?} git_obj", git_obj_models.len());
                if let Some(repo_name) = client_paras.pending_git_obj_download.get(&request_id) {
                    let path = get_repo_full_path(repo_name);
                    let pack_protocol =
                        get_pack_protocol(&path, client_paras.storage.clone()).await;
                    let git_obj_active_model = git_obj_models
                        .iter()
                        .map(|m| git_obj::ActiveModel {
                            id: Set(m.id),
                            git_id: Set(m.git_id.clone()),
                            object_type: Set(m.object_type.clone()),
                            data: Set(m.data.clone()),
                        })
                        .collect();
                    match pack_protocol
                        .storage
                        .save_obj_data(git_obj_active_model)
                        .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                "Save {:?} git_obj to database successfully",
                                git_obj_models.len()
                            );
                        }
                        Err(e) => {
                            tracing::error!("{:?}", e);
                        }
                    }
                    client_paras.pending_git_obj_download.remove(&request_id);
                }
            }
        },
        request_response::Event::OutboundFailure { peer, error, .. } => {
            tracing::error!("Git object  outbound failure: {:?},{:?}", peer, error);
        }
        event => {
            tracing::debug!("Request_response event:{:?}", event);
        }
    }
}

async fn git_upload_pack_handler(
    path: &str,
    client_paras: &mut ClientParas,
    want: HashSet<String>,
    have: HashSet<String>,
) -> Result<(Vec<u8>, String), String> {
    let pack_protocol = get_pack_protocol(path, client_paras.storage.clone()).await;
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
