use super::behaviour;
use crate::network::behaviour::{FileRequest, FileResponse};
use crate::node::client::ClientParas;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{
    AddProviderOk, GetClosestPeersOk, GetProvidersOk, GetRecordOk, Kademlia, KademliaEvent,
    PeerRecord, PutRecordOk, QueryResult,
};
use libp2p::{identify, multiaddr, rendezvous, request_response, Swarm};
use std::fs;
use std::io::Write;

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
                                    .with(multiaddr::Protocol::P2p(*peer.as_ref())),
                            )
                            .unwrap();
                    }
                }
            }
        }
        rendezvous::client::Event::RegisterFailed(error) => {
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

pub fn file_transfer_event_handler(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    event: request_response::Event<FileRequest, FileResponse>,
) {
    match event {
        request_response::Event::Message { message, .. } => match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                //receive file transfer request
                tracing::info!(
                    "File transfer event request, {:?}, channel:{:?}",
                    request,
                    channel
                );
                //check if we provide this file
                let file_name = request.0;
                let file_path = match client_paras.file_provide_map.get(&file_name) {
                    Some(s) => s,
                    None => {
                        tracing::error!("File path not exists");
                        return;
                    }
                };
                let file_content = match read_file_to_vec(file_path) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("File open failed  :{:?}", e);
                        return;
                    }
                };
                let result = swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, FileResponse(file_content));
                match result {
                    Ok(()) => {
                        tracing::info!(
                            "File send success, fileName:{:?}, filePath:{:?}",
                            file_name,
                            file_path
                        );
                    }
                    Err(e) => {
                        tracing::error!("File request response Error :{:?}", e);
                    }
                }
            }
            request_response::Message::Response {
                request_id,
                response,
            } => {
                // get a file
                tracing::info!("File transfer event response, request_id: {:?}", request_id,);
                if let Some(file_name) = client_paras.pending_get_file.get(&request_id) {
                    let file_content = response.0;
                    let file_path = format!("download/{}", file_name);
                    tracing::info!("Download file {} to {}", file_name, file_path);
                    if let Err(e) = write_vec_to_file(&file_path, &file_content) {
                        tracing::info!("Save file failed: {} ", e);
                    }
                }
            }
        },

        event => {
            tracing::debug!("Request_response event:{:?}", event);
        }
    }
}

//After accepting the event, a new event needs to be triggered
pub fn kad_event_callback(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    event: KademliaEvent,
) {
    match event {
        KademliaEvent::OutboundQueryProgressed {
            id,
            result: QueryResult::GetProviders(Ok(GetProvidersOk::FoundProviders { providers, .. })),
            ..
        } => {
            //check if there is pending_get_file
            if let Some(file_name) = client_paras.pending_request_file.get(&id) {
                if providers.is_empty() {
                    tracing::error!("Cannot find file {} from p2p network", file_name,);
                }
                //Trigger file download request
                for peer_id in providers {
                    let request_file_id = swarm
                        .behaviour_mut()
                        .request_response
                        .send_request(&peer_id, FileRequest(file_name.to_string()));
                    tracing::info!("Get File {} from peer {}", file_name, peer_id);
                    client_paras
                        .pending_get_file
                        .insert(request_file_id, file_name.clone());
                }
            }
            client_paras.pending_request_file.remove(&id);
        }
        _event => {}
    }
}

fn read_file_to_vec(filepath: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let data = fs::read(filepath)?;
    Ok(data)
}

fn write_vec_to_file(path: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(path);
    if let Some(prefix) = path.parent() {
        if !prefix.exists() {
            fs::create_dir_all(prefix)?;
        }
    }
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    let remaining = file.write(data)?;
    if remaining > 0 {}
    Ok(())
}
