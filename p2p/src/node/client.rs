use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use async_std::io;
use async_std::io::prelude::BufReadExt;
use futures::executor::block_on;
use futures::{future::FutureExt, stream::StreamExt};
use libp2p::identity::secp256k1::SecretKey;
use libp2p::kad::store::MemoryStore;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::SwarmEvent;
use libp2p::{
    dcutr, identify, identity, kad, multiaddr, noise, rendezvous, request_response, tcp, yamux,
    Multiaddr, PeerId, StreamProtocol,
};
use tokio::sync::Mutex;

use common::enums::DataSource;
use entity::objects::Model;
use storage::driver::database;

use crate::cbor;
use crate::network::behaviour::{self, Behaviour, Event};
use crate::network::event_handler;
use crate::node::client_http;
use crate::node::input_command;
use crate::node::ClientParas;

pub async fn run(
    sk: SecretKey,
    p2p_address: String,
    bootstrap_node: String,
    data_source: DataSource,
) -> Result<(), Box<dyn Error>> {
    //secp256k1 keypair
    let secp = secp256k1::Secp256k1::new();
    let secret_key = secp256k1::SecretKey::from_slice(&sk.to_bytes()).unwrap();
    let key_pair = secp256k1::KeyPair::from_secret_key(&secp, &secret_key);

    //libp2p keypair with same sk
    let secp256k1_kp = identity::secp256k1::Keypair::from(sk.clone());
    let local_key = identity::Keypair::from(secp256k1_kp);
    let local_peer_id = PeerId::from(local_key.public());
    tracing::info!("Local peer id: {local_peer_id:?}");

    let store = MemoryStore::new(local_peer_id);

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_async_std()
        .with_tcp(
            tcp::Config::default().port_reuse(true),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|keypair, relay_behaviour| Behaviour {
            relay_client: relay_behaviour,
            identify: identify::Behaviour::new(identify::Config::new(
                "/mega/0.0.1".to_string(),
                keypair.public(),
            )),
            dcutr: dcutr::Behaviour::new(keypair.public().to_peer_id()),
            //DHT
            kademlia: kad::Behaviour::new(keypair.public().to_peer_id(), store),
            //discover
            rendezvous: rendezvous::client::Behaviour::new(keypair.clone()),
            // git pull, git clone
            git_upload_pack: cbor::Behaviour::new(
                [(
                    StreamProtocol::new("/mega/git_upload_pack"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default().with_request_timeout(Duration::from_secs(100)),
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
                request_response::Config::default().with_request_timeout(Duration::from_secs(100)),
            ),
            nostr: cbor::Behaviour::new(
                [(StreamProtocol::new("/mega/nostr"), ProtocolSupport::Full)],
                request_response::Config::default(),
            ),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(100)))
        .build();

    // Listen on all interfaces
    swarm.listen_on(p2p_address.parse()?)?;

    tracing::info!("Connect to database");
    let storage = database::init(&data_source).await;
    let mut client_paras = ClientParas {
        cookie: None,
        rendezvous_point: None,
        bootstrap_node_addr: None,
        storage,
        key_pair,
        pending_git_upload_package: HashMap::new(),
        pending_git_pull: HashMap::new(),
        pending_git_obj_download: HashMap::new(),
        pending_repo_info_update_fork: HashMap::new(),
        pending_repo_info_search_to_download_obj: HashMap::new(),
        pending_git_obj_id_download: HashMap::new(),
        repo_node_list: HashMap::new(),
        repo_id_need_list: HashMap::<String, Vec<String>>::new(),
        repo_receive_git_obj_model_list: HashMap::<String, Vec<Model>>::new(),
    };
    // Wait to listen on all interfaces.
    block_on(async {
        let mut delay = futures_timer::Delay::new(Duration::from_secs(1)).fuse();
        loop {
            futures::select! {
                event = swarm.next() => {
                    match event.unwrap() {
                        SwarmEvent::NewListenAddr { address, .. } => {
                           tracing::info!("Listening on {:?}", address);
                        }
                        event => panic!("{event:?}"),
                    }
                }
                _ = delay => {
                    // Likely listening on all interfaces now, thus continuing by breaking the loop.
                    break;
                }
            }
        }
    });

    //dial to bootstrap_node
    if !bootstrap_node.is_empty() {
        let bootstrap_node_addr: Multiaddr = bootstrap_node.parse()?;
        tracing::info!("Trying to dial bootstrap node{:?}", bootstrap_node_addr);
        swarm.dial(bootstrap_node_addr.clone())?;
        block_on(async {
            let mut learned_observed_addr = false;
            let mut told_relay_observed_addr = false;
            let mut relay_peer_id: Option<PeerId> = None;
            let mut delay = futures_timer::Delay::new(Duration::from_secs(10)).fuse();
            loop {
                futures::select! {
                    event = swarm.next() => {
                        match event.unwrap() {
                            SwarmEvent::NewListenAddr { .. } => {}
                            SwarmEvent::Dialing{peer_id, ..} => {
                                tracing::info!("Dialing: {:?}", peer_id)
                            },
                            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                client_paras.rendezvous_point.replace(peer_id);
                                let p2p_suffix = multiaddr::Protocol::P2p(peer_id);
                                let bootstrap_node_addr =
                                    if !bootstrap_node_addr.ends_with(&Multiaddr::empty().with(p2p_suffix.clone())) {
                                        bootstrap_node_addr.clone().with(p2p_suffix)
                                    } else {
                                        bootstrap_node_addr.clone()
                                    };
                                client_paras.bootstrap_node_addr.replace(bootstrap_node_addr.clone());
                                swarm.behaviour_mut().kademlia.add_address(&peer_id.clone(),bootstrap_node_addr.clone());
                                tracing::info!("ConnectionEstablished:{} at {}", peer_id, bootstrap_node_addr);
                            },
                            SwarmEvent::Behaviour(behaviour::Event::Identify(identify::Event::Sent {
                                ..
                            })) => {
                                tracing::info!("Told Bootstrap Node our public address.");
                                told_relay_observed_addr = true;
                            },
                            SwarmEvent::Behaviour(Event::Identify(
                                identify::Event::Received {
                                    info ,peer_id
                                },
                            )) => {
                                tracing::info!("Bootstrap Node told us our public address: {:?}", info.observed_addr);
                                learned_observed_addr = true;
                                relay_peer_id.replace(peer_id);
                            },
                            event => tracing::info!("{:?}", event),
                        }
                         if learned_observed_addr && told_relay_observed_addr {
                            //success connect to bootstrap node
                            tracing::info!("Dial bootstrap node successfully");
                            if let Some(bootstrap_node_addr) = client_paras.bootstrap_node_addr.clone(){
                                let public_addr = bootstrap_node_addr.with(multiaddr::Protocol::P2pCircuit);
                                swarm.listen_on(public_addr.clone()).unwrap();
                                swarm.add_external_address(public_addr);
                                //register rendezvous
                                if let Err(error) = swarm.behaviour_mut().rendezvous.register(
                                    rendezvous::Namespace::from_static(event_handler::NAMESPACE),
                                    relay_peer_id.unwrap(),
                                    None,
                                ){
                                    tracing::error!("Failed to register: {error}");
                                }
                            }
                            break;
                        }
                    }
                    _ = delay => {
                        tracing::error!("Dial bootstrap node failed: Timeout");
                        break;
                    }
                }
            }
        });
    }

    let swarm = Arc::new(Mutex::new(swarm));
    let client_paras = Arc::new(Mutex::new(client_paras));

    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    let server_task = tokio::spawn(client_http::server(
        swarm.clone(),
        client_paras.clone(),
    ));

    let loop_task = tokio::spawn(async move {
        loop {
            let mut delay = futures_timer::Delay::new(Duration::from_secs(1)).fuse();
            let mut swarm_lock = swarm.lock().await;
            futures::select! {
                line = stdin.select_next_some() => {
                    drop(swarm_lock);
                    let line :String = line.expect("Stdin not to close");
                    if line.is_empty() {
                            continue;
                    }
                    //kad input
                    input_command::handle_input_command(swarm.clone(), client_paras.clone(), line.to_string()).await;
                },
                event = swarm_lock.select_next_some() => {
                    let mut client_paras_lock = client_paras.lock().await;
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!("Listening on {:?}", address);
                        }
                        SwarmEvent::ConnectionEstablished {
                            peer_id, endpoint, ..
                            } => {
                                tracing::info!("Established connection to {:?} via {:?}", peer_id, endpoint);
                                swarm_lock.behaviour_mut().kademlia.add_address(&peer_id,endpoint.get_remote_address().clone());
                                let peers = swarm_lock.connected_peers();
                                for p in peers {
                                    tracing::info!("Connected peer: {}",p);
                                };
                            },
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            tracing::info!("Disconnect {:?}", peer_id);
                        },
                        SwarmEvent::OutgoingConnectionError{error,..} => {
                            tracing::debug!("OutgoingConnectionError {:?}", error);
                        },
                        //Identify events
                        SwarmEvent::Behaviour(Event::Identify(event)) => {
                            event_handler::identify_event_handler(&mut swarm_lock.behaviour_mut().kademlia, event);
                        },
                        //RendezvousClient events
                        SwarmEvent::Behaviour(Event::Rendezvous(event)) => {
                            event_handler::rendezvous_client_event_handler(&mut swarm_lock, &mut client_paras_lock, event);
                        },
                        //kad events
                        SwarmEvent::Behaviour(Event::Kademlia(event)) => {
                            event_handler::kad_event_handler(&mut swarm_lock, &mut client_paras_lock, event).await;
                        },
                        //GitUploadPack events
                        SwarmEvent::Behaviour(Event::GitUploadPack(event)) => {
                             event_handler::git_upload_pack_event_handler(&mut swarm_lock, &mut client_paras_lock, event).await;
                        },
                        //GitInfoRefs events
                        SwarmEvent::Behaviour(Event::GitInfoRefs(event)) => {
                             event_handler::git_info_refs_event_handler(&mut swarm_lock, &mut client_paras_lock, event).await;
                        },
                         //GitObject events
                        SwarmEvent::Behaviour(Event::GitObject(event)) => {
                             event_handler::git_object_event_handler(&mut swarm_lock, &mut client_paras_lock, event).await;
                        },
                        //Nostr events
                        SwarmEvent::Behaviour(Event::Nostr(event)) => {
                             event_handler::nostr_event_handler(&mut swarm_lock, &mut client_paras_lock, event).await;
                        },
                        _ => {
                            tracing::debug!("Event: {:?}", event);
                        }
                    };
                    drop(swarm_lock);
                    drop(client_paras_lock);
                },
                _ = delay => {
                    drop(swarm_lock);
                    continue;
                }
            }
        }
    });
    tokio::try_join!(server_task, loop_task).unwrap();
    Ok(())
}
