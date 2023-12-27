use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

use async_std::io;
use async_std::io::prelude::BufReadExt;
use futures::executor::block_on;
use futures::stream::StreamExt;
use kvcache::connector::redis::RedisClient;
use kvcache::KVCache;
use libp2p::kad::store::RecordStore;
use libp2p::kad::{
    AddProviderOk, GetClosestPeersOk, GetProvidersOk, GetRecordOk, PeerRecord, PutRecordOk,
    QueryResult, Quorum, Record,
};
use libp2p::request_response::{cbor, ProtocolSupport};
use libp2p::{
    identify, identity,
    identity::PeerId,
    kad, noise, relay, rendezvous, request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, StreamProtocol, Swarm,
};

use crate::internal::dht_redis_store::DHTRedisStore;
use crate::network::event_handler;
use crate::nostr::client_message::{ClientMessage, SubscriptionId};
use crate::nostr::event::{GitEvent, NostrEvent};
use crate::nostr::relay_message::RelayMessage;
use crate::nostr::tag::TagKind;
use crate::nostr::{NostrReq, NostrRes};

#[derive(NetworkBehaviour)]
pub struct ServerBehaviour<TStore: RecordStore> {
    pub relay: relay::Behaviour,
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<TStore>,
    pub rendezvous: rendezvous::server::Behaviour,
    pub nostr: cbor::Behaviour<NostrReq, NostrRes>,
}

pub fn run(local_key: identity::Keypair, p2p_address: String) -> Result<(), Box<dyn Error>> {
    let local_peer_id = PeerId::from(local_key.public());
    tracing::info!("Local peer id: {local_peer_id:?}");

    let redis_store = DHTRedisStore::new();
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_async_std()
        .with_tcp(
            tcp::Config::default().port_reuse(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| ServerBehaviour {
            relay: relay::Behaviour::new(key.public().to_peer_id(), Default::default()),
            identify: identify::Behaviour::new(identify::Config::new(
                "/mega/0.0.1".to_string(),
                key.public(),
            )),
            kademlia: kad::Behaviour::new(key.public().to_peer_id(), redis_store),
            rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
            nostr: cbor::Behaviour::new(
                [(StreamProtocol::new("/mega/nostr"), ProtocolSupport::Full)],
                request_response::Config::default(),
            ),
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();

    // Listen on all interfaces
    swarm.listen_on(p2p_address.parse()?)?;

    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    block_on(async {
        loop {
            futures::select! {
                line = stdin.select_next_some() => {
                    let line :String = line.expect("Stdin not to close");
                    if line.is_empty() {
                        continue;
                    }
                    //kad input
                    handle_kad_command(&mut swarm.behaviour_mut().kademlia,line.to_string().split_whitespace().collect());
                },
                event = swarm.select_next_some() => match event{
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Identify(identify::Event::Received {
                        info,peer_id
                    })) => {
                        swarm.add_external_address(info.observed_addr.clone());
                        for listen_addr in info.listen_addrs.clone(){
                            swarm.behaviour_mut().kademlia.add_address(&peer_id.clone(),listen_addr);
                        }
                        tracing::info!("Identify Event Received, peer_id :{}, info:{:?}", peer_id, info);
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        tracing::info!("Listening on {address:?}");
                    }
                    //kad events
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Kademlia(event)) => {
                         kad_event_handler(event);
                    }
                    //RendezvousServer events
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Rendezvous(event)) => {
                        event_handler::rendezvous_server_event_handler(event);
                    },
                     //Nostr events
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Nostr(event)) => {
                         nostr_relay_event_handler(&mut swarm,event);
                    },
                    _ => {
                        tracing::debug!("Event: {:?}", event);
                    }
                }
            }
        }
    });

    Ok(())
}

pub fn kad_event_handler(event: kad::Event) {
    if let kad::Event::OutboundQueryProgressed { result, .. } = event {
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

pub fn handle_kad_command(kademlia: &mut kad::Behaviour<DHTRedisStore>, args: Vec<&str>) {
    let mut args_iter = args.iter().copied();
    match args_iter.next() {
        Some("get") => {
            let key = {
                match args_iter.next() {
                    Some(key) => kad::RecordKey::new(&key),
                    None => {
                        eprintln!("Expected key");
                        return;
                    }
                }
            };
            kademlia.get_record(key);
        }
        Some("put") => {
            let key = {
                match args_iter.next() {
                    Some(key) => kad::RecordKey::new(&key),
                    None => {
                        eprintln!("Expected key");
                        return;
                    }
                }
            };
            let value = {
                match args_iter.next() {
                    Some(value) => value.as_bytes().to_vec(),
                    None => {
                        eprintln!("Expected value");
                        return;
                    }
                }
            };
            let record = Record {
                key,
                value,
                publisher: None,
                expires: None,
            };
            if let Err(e) = kademlia.put_record(record, Quorum::One) {
                eprintln!("Put record failed :{}", e);
            }
        }
        _ => {
            eprintln!("expected command: get, put");
        }
    }
}

pub fn nostr_relay_event_handler(
    swarm: &mut Swarm<ServerBehaviour<DHTRedisStore>>,
    event: request_response::Event<NostrReq, NostrRes>,
) {
    match event {
        request_response::Event::Message { peer, message, .. } => match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                tracing::info!(
                    "Nostr relay receive request:\n {} \nfrom {}",
                    request.0,
                    peer
                );

                let message = match parsed_nostr_client_request(request.0) {
                    Ok(message) => message,
                    Err(e) => {
                        let msg = RelayMessage::new_notice(e.clone());
                        let _ = swarm
                            .behaviour_mut()
                            .nostr
                            .send_response(channel, NostrRes(msg.as_json()));
                        return;
                    }
                };

                match message {
                    //relay receive subscription
                    ClientMessage::Req { filters, .. } => {
                        let mut subscribe_repo_names: Vec<String> = Vec::new();
                        filters.iter().for_each(|f| {
                            let tag_name = TagKind::RepoName.to_string();
                            if let Some(repo_names) = f.generic_tags.get(tag_name.as_str()) {
                                repo_names
                                    .iter()
                                    .for_each(|n| subscribe_repo_names.push(n.to_string()));
                            };
                            subscribe_repo_names.iter().for_each(|subscribe_repo_name| {
                                let key = add_subscribe_key_prefix(subscribe_repo_name.clone());
                                redis_update_vec(key, peer.to_string());
                            });
                        });
                        let msg = RelayMessage::new_notice(String::from("Subscribe successfully"));
                        let _ = swarm
                            .behaviour_mut()
                            .nostr
                            .send_response(channel, NostrRes(msg.as_json()));
                    }
                    //relay receive event
                    ClientMessage::Event { 0: nostr_event, .. } => {
                        //verify
                        if let Err(e) = nostr_event.verify() {
                            let msg = RelayMessage::new_notice(e.to_string());
                            let _ = swarm
                                .behaviour_mut()
                                .nostr
                                .send_response(channel, NostrRes(msg.as_json()));
                            return;
                        }
                        //save event
                        let git_event = GitEvent::from_tags(nostr_event.tags.clone());
                        let key = nostr_event_key_prefix(git_event.repo_name.clone());
                        redis_update_vec(key, nostr_event.as_json());

                        //reply to client
                        let relay_msg =
                            RelayMessage::new_ok(nostr_event.id.clone(), true, "".to_string());
                        let _ = swarm
                            .behaviour_mut()
                            .nostr
                            .send_response(channel, NostrRes(relay_msg.as_json()));

                        //broadcast event to subscribers
                        broadcast_event_to_subscribers(swarm, git_event.repo_name, *nostr_event);
                    }
                }
            }
            request_response::Message::Response { response, .. } => {
                tracing::info!(
                    "Nostr relay receive response, \n {} \nfrom {}",
                    response.0,
                    peer
                );
            }
        },
        request_response::Event::OutboundFailure { peer, error, .. } => {
            tracing::error!("nostr outbound failure: {:?},{:?}", peer, error);
        }
        event => {
            tracing::debug!("Request_response event:{:?}", event);
        }
    }
}

fn add_subscribe_key_prefix(key: String) -> String {
    let prefix = "nostr_subscription_";
    format!("{}{}", prefix, key)
}

fn nostr_event_key_prefix(key: String) -> String {
    let prefix = "nostr_event_";
    format!("{}{}", prefix, key)
}

fn redis_update_vec(key: String, value: String) {
    let redis_cache = KVCache::<RedisClient<String, String>>::new();
    if let Some(v) = redis_cache.get(key.clone()) {
        let mut vec: Vec<String> = serde_json::from_str(v.as_str()).unwrap();
        if !vec.contains(&value) {
            vec.push(value);
            let value = serde_json::to_string(&vec).unwrap();
            redis_cache.set(key, value).unwrap();
        }
    } else {
        let vec = vec![value.to_string()];
        let value = serde_json::to_string(&vec).unwrap();
        redis_cache.set(key, value).unwrap();
    }
}

fn parsed_nostr_client_request(request: String) -> Result<ClientMessage, String> {
    let parsed = if let Ok(parsed) = serde_json::from_str(request.as_str()) {
        parsed
    } else {
        let s = String::from("Invalid message format!");
        return Err(s);
    };
    let message = if let Ok(message) = ClientMessage::from_value(parsed) {
        message
    } else {
        let s = String::from("Invalid message format!");
        return Err(s);
    };
    Ok(message)
}

fn broadcast_event_to_subscribers(
    swarm: &mut Swarm<ServerBehaviour<DHTRedisStore>>,
    repo_name: String,
    nostr_event: NostrEvent,
) {
    let redis_cache = KVCache::<RedisClient<String, String>>::new();
    let subscribe_key = add_subscribe_key_prefix(repo_name);
    if let Some(v) = redis_cache.get(subscribe_key) {
        let vec: Vec<String> = serde_json::from_str(v.as_str()).unwrap();
        for peer_id in vec {
            let msg = RelayMessage::new_event(SubscriptionId::generate(), nostr_event.clone());
            let _ = swarm.behaviour_mut().nostr.send_request(
                &PeerId::from_str(peer_id.as_str()).unwrap(),
                NostrReq(msg.as_json()),
            );
        }
    }
}
