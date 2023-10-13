use super::behaviour;
use crate::network::behaviour::{
    GitInfoRefsReq, GitInfoRefsRes, GitObjectReq, GitObjectRes, GitUploadPackReq, GitUploadPackRes,
};
use crate::node::{get_utc_timestamp, ClientParas, Fork, MegaRepoInfo};
use crate::{get_pack_protocol, get_repo_full_path};
use bytes::Bytes;
use common::utils;
use entity::git_obj::Model;
use git::protocol::RefCommand;
use git::structure::conversion;
use libp2p::kad::record::Key;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{
    AddProviderOk, GetClosestPeersOk, GetProvidersOk, GetRecordOk, Kademlia, KademliaEvent,
    PeerRecord, PutRecordOk, QueryResult, Quorum, Record,
};
use libp2p::{identify, multiaddr, rendezvous, request_response, PeerId, Swarm};
use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;

pub const NAMESPACE: &str = "rendezvous_mega";

pub async fn kad_event_handler(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    event: KademliaEvent,
) {
    if let KademliaEvent::OutboundQueryProgressed { id, result, .. } = event {
        match result {
            QueryResult::GetRecord(Ok(GetRecordOk::FoundRecord(PeerRecord { record, peer }))) => {
                let peer_id = match peer {
                    Some(id) => id.to_string(),
                    None => "local".to_string(),
                };
                tracing::info!(
                    "Got record key[{}]={},from {}",
                    String::from_utf8(record.key.to_vec()).unwrap(),
                    String::from_utf8(record.value.clone()).unwrap(),
                    peer_id
                );
                if let Some(object_id) = client_paras.pending_repo_info_update_fork.get(&id) {
                    tracing::info!("update repo info forks");
                    // update repo info forks
                    if let Ok(p) = serde_json::from_slice(&record.value) {
                        let mut repo_info: MegaRepoInfo = p;
                        let local_peer_id = swarm.local_peer_id().to_string();
                        let fork = Fork {
                            peer: local_peer_id.clone(),
                            latest: object_id.clone(),
                            timestamp: get_utc_timestamp(),
                        };
                        repo_info.forks.retain(|r| r.peer != local_peer_id);
                        repo_info.forks.push(fork);
                        let record = Record {
                            key: Key::new(&repo_info.name),
                            value: serde_json::to_vec(&repo_info).unwrap(),
                            publisher: None,
                            expires: None,
                        };
                        if let Err(e) = swarm
                            .behaviour_mut()
                            .kademlia
                            .put_record(record, Quorum::One)
                        {
                            eprintln!("Failed to store record:{}", e);
                        }
                    }
                    client_paras.pending_repo_info_update_fork.remove(&id);
                } else if let Some(repo_name) = client_paras
                    .pending_repo_info_search_to_download_obj
                    .clone()
                    .get(&id)
                {
                    //try to search origin node
                    tracing::info!("try to get origin node to search git_obj_id_list");
                    if let Ok(p) = serde_json::from_slice(&record.value) {
                        let repo_info: MegaRepoInfo = p;
                        //save all node that have this repo,the first one is origin
                        let mut node_id_list: Vec<String> = Vec::new();
                        node_id_list.push(repo_info.origin.clone());
                        for fork in &repo_info.forks {
                            node_id_list.push(fork.peer.clone());
                        }
                        client_paras
                            .repo_node_list
                            .insert(repo_name.clone(), node_id_list);
                        let remote_peer_id = PeerId::from_str(&repo_info.origin).unwrap();
                        let path = get_repo_full_path(repo_name);
                        //to get local git_obj id
                        let local_git_ids = get_all_git_obj_ids(&path, client_paras).await;
                        let request_file_id = swarm
                            .behaviour_mut()
                            .git_info_refs
                            .send_request(&remote_peer_id, GitInfoRefsReq(path, local_git_ids));
                        client_paras
                            .pending_git_obj_id_download
                            .insert(request_file_id, repo_name.to_string());
                    }
                }
                client_paras
                    .pending_repo_info_search_to_download_obj
                    .remove(&id);
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
                    let old_object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
                    tracing::info!(
                        "new_object_id:{}; old_object_id:{}",
                        object_id.clone(),
                        old_object_id
                    );
                    let command = RefCommand::new(
                        old_object_id,
                        object_id.clone(),
                        String::from("refs/heads/master"),
                    );
                    pack_protocol.command_list.push(command);
                    let result = pack_protocol
                        .git_receive_pack(Bytes::from(package_data))
                        .await;
                    match result {
                        Ok(_) => {
                            tracing::info!("Save git package successfully :{}", repo_name);
                            //update repoInfo
                            let kad_query_id = swarm
                                .behaviour_mut()
                                .kademlia
                                .get_record(Key::new(&repo_name));
                            client_paras
                                .pending_repo_info_update_fork
                                .insert(kad_query_id, object_id);
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
                let git_ids_they_have = request.1;
                tracing::info!("git_ids_they_have: {:?}", git_ids_they_have);
                let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
                let ref_git_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
                let mut git_obj_ids = get_all_git_obj_ids(&path, client_paras).await;
                if !git_ids_they_have.is_empty() {
                    git_obj_ids.retain(|id| !git_ids_they_have.contains(id));
                }
                tracing::info!("git_ids_they_need: {:?}", git_obj_ids);
                let _ = swarm
                    .behaviour_mut()
                    .git_info_refs
                    .send_response(channel, GitInfoRefsRes(ref_git_id, git_obj_ids));
            }
            request_response::Message::Response {
                request_id,
                response,
            } => {
                //receive git info refs  response
                tracing::info!("Response git info refs event, request_id: {:?}", request_id);
                if let Some(repo_name) = client_paras.pending_git_pull.get(&request_id) {
                    //have git_ids and try to send pull request
                    // mega pull and mega clone
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
                if let Some(repo_name) = client_paras
                    .pending_git_obj_id_download
                    .clone()
                    .get(&request_id)
                {
                    // have git_ids and try to download git obj
                    // git clone-obj and git pull-obj
                    let _ref_git_id = response.0;
                    let git_ids_need = response.1;
                    let path = get_repo_full_path(repo_name);
                    tracing::info!("path: {}", path);
                    tracing::info!("git_ids_need: {:?}", git_ids_need);
                    //trying to download git_obj from peers
                    if let Some(r) = client_paras.repo_node_list.clone().get(repo_name) {
                        let mut repo_list = r.clone();
                        if !repo_list.is_empty() {
                            repo_list
                                .retain(|r| *r != swarm.local_peer_id().to_string());
                            tracing::info!("try to download git object from: {:?}", repo_list);
                            tracing::info!("the origin is: {}", repo_list[0]);
                            // Try to download separately
                            let split_git_ids = split_array(git_ids_need.clone(), repo_list.len());
                            let repo_id_need_list_arc = client_paras.repo_id_need_list.clone();
                            {
                                let mut repo_id_need_list = repo_id_need_list_arc.lock().unwrap();
                                repo_id_need_list.insert(repo_name.to_string(), git_ids_need);
                            }

                            for i in 0..repo_list.len() {
                                // send get git object request
                                let ids = split_git_ids[i].clone();
                                let repo_peer_id = PeerId::from_str(&repo_list[i].clone()).unwrap();
                                let new_request_id = swarm
                                    .behaviour_mut()
                                    .git_object
                                    .send_request(&repo_peer_id, GitObjectReq(path.clone(), ids));
                                client_paras
                                    .pending_git_obj_download
                                    .insert(new_request_id, repo_name.to_string());
                            }
                        }
                    }
                    client_paras.pending_git_obj_id_download.remove(&request_id);
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
        request_response::Event::Message { peer, message, .. } => match message {
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
                tracing::debug!("Response git object event, request_id: {:?}", request_id);
                let git_obj_models = response.0;
                tracing::info!(
                    "Receive {:?} git_obj, from {:?}",
                    git_obj_models.len(),
                    peer
                );
                let receive_id_list: Vec<String> = git_obj_models
                    .clone()
                    .iter()
                    .map(|m| m.git_id.clone())
                    .collect();
                tracing::info!("git_obj_id_list:{:?}", receive_id_list);

                if let Some(repo_name) = client_paras.pending_git_obj_download.get(&request_id) {
                    let repo_receive_git_obj_model_list_arc =
                        client_paras.repo_receive_git_obj_model_list.clone();
                    {
                        let mut receive_git_obj_model_map =
                            repo_receive_git_obj_model_list_arc.lock().unwrap();
                        receive_git_obj_model_map
                            .entry(repo_name.clone())
                            .or_default();
                        let receive_obj_model_list =
                            receive_git_obj_model_map.get(repo_name).unwrap();
                        let mut clone = receive_obj_model_list.clone();
                        clone.append(&mut git_obj_models.clone());
                        tracing::info!("receive_obj_model_list:{:?}", clone.len());
                        receive_git_obj_model_map.insert(repo_name.to_string(), clone);
                    }

                    let repo_id_need_list_arc = client_paras.repo_id_need_list.clone();
                    let mut finish = false;
                    {
                        let mut repo_id_need_list_map = repo_id_need_list_arc.lock().unwrap();
                        if let Some(id_need_list) = repo_id_need_list_map.get(repo_name) {
                            let mut clone = id_need_list.clone();
                            clone.retain(|x| !receive_id_list.contains(x));
                            if clone.is_empty() {
                                finish = true;
                            }
                            repo_id_need_list_map.insert(repo_name.to_string(), clone);
                        }
                    }
                    println!("finish:{}", finish);
                    if finish {
                        let repo_receive_git_obj_model_list_arc2 =
                            client_paras.repo_receive_git_obj_model_list.clone();
                        let mut obj_model_list: Vec<Model> = Vec::new();
                        {
                            let mut receive_git_obj_model_map =
                                repo_receive_git_obj_model_list_arc2.lock().unwrap();
                            if !receive_git_obj_model_map.contains_key(repo_name) {
                                tracing::error!("git_object cache error");
                                return;
                            }
                            let receive_git_obj_model =
                                receive_git_obj_model_map.get(repo_name).unwrap();
                            obj_model_list.append(&mut receive_git_obj_model.clone());
                            receive_git_obj_model_map.remove(repo_name);
                        }
                        tracing::info!("receive all git_object :{:?}", obj_model_list.len());
                        let path = get_repo_full_path(repo_name);
                        match conversion::save_node_from_git_obj(
                            client_paras.storage.clone(),
                            Path::new(&path),
                            obj_model_list.clone(),
                        )
                        .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Save {:?} git_obj to database successfully",
                                    obj_model_list.len()
                                );
                                let path = get_repo_full_path(repo_name);
                                let pack_protocol =
                                    get_pack_protocol(&path, client_paras.storage.clone()).await;
                                let object_id =
                                    pack_protocol.get_head_object_id(Path::new(&path)).await;
                                //update repoInfo
                                let kad_query_id = swarm
                                    .behaviour_mut()
                                    .kademlia
                                    .get_record(Key::new(&repo_name));
                                client_paras
                                    .pending_repo_info_update_fork
                                    .insert(kad_query_id, object_id);
                            }
                            Err(e) => {
                                tracing::error!("{:?}", e);
                            }
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

async fn get_all_git_obj_ids(path: &str, client_paras: &mut ClientParas) -> Vec<String> {
    let pack_protocol = get_pack_protocol(path, client_paras.storage.clone()).await;
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

fn split_array(a: Vec<String>, count: usize) -> Vec<Vec<String>> {
    let mut result = vec![];
    let split_num = a.len() / count;
    for i in 0..count {
        let v: Vec<_> = if i != count - 1 {
            a.clone()
                .drain(i * split_num..(i + 1) * split_num)
                .collect()
        } else {
            a.clone().drain(i * split_num..).collect()
        };
        result.push(v);
    }
    result
}
