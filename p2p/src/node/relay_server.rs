use crate::network::event_handler;
use async_std::io;
use async_std::io::prelude::BufReadExt;
use futures::executor::block_on;
use futures::stream::StreamExt;
use libp2p::kad::store::{RecordStore};
use libp2p::kad::{AddProviderOk, GetClosestPeersOk, GetProvidersOk, GetRecordOk, PeerRecord, PutRecordOk, QueryResult, Quorum, Record};
use libp2p::{
    identify, identity,
    identity::PeerId,
    kad, noise, relay, rendezvous,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use std::error::Error;
use std::time::Duration;
use crate::internal::dht_redis_store::DHTRedisStore;

#[derive(NetworkBehaviour)]
pub struct ServerBehaviour<TStore: RecordStore> {
    pub relay: relay::Behaviour,
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<TStore>,
    pub rendezvous: rendezvous::server::Behaviour,
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