use super::lib;
use async_std::io;
use async_std::io::prelude::BufReadExt;
use futures::executor::block_on;
use futures::{future::FutureExt, stream::StreamExt};
use libp2p::core::upgrade;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::rendezvous::Cookie;
use libp2p::swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent};
use libp2p::{
    dcutr, identify, identity, multiaddr, noise, relay, rendezvous, tcp, yamux, Multiaddr, PeerId,
    Transport,
};
use std::error::Error;

pub fn run(
    local_key: identity::Keypair,
    p2p_address: String,
    bootstrap_node: String,
) -> Result<(), Box<dyn Error>> {
    let local_peer_id = PeerId::from(local_key.public());
    tracing::info!("Local peer id: {local_peer_id:?}");

    let (relay_transport, client) = relay::client::new(local_peer_id);

    let tcp_transport = relay_transport
        .or_transport(tcp::async_io::Transport::new(
            tcp::Config::default().port_reuse(true),
        ))
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::Config::new(&local_key).unwrap())
        .multiplex(yamux::Config::default())
        .boxed();

    let store = MemoryStore::new(local_peer_id);

    let behaviour = Behaviour {
        relay_client: client,
        identify: identify::Behaviour::new(identify::Config::new(
            "/mega/0.0.1".to_string(),
            local_key.public(),
        )),
        dcutr: dcutr::Behaviour::new(local_peer_id),
        kademlia: Kademlia::new(local_peer_id, store),
        rendezvous: rendezvous::client::Behaviour::new(local_key),
    };
    let mut swarm = SwarmBuilder::without_executor(tcp_transport, behaviour, local_peer_id).build();

    // Listen on all interfaces
    swarm.listen_on(p2p_address.parse()?)?;

    let mut client_paras = ClientParas {
        cookie: None,
        rendezvous_point: None,
        bootstrap_node_addr: None,
    };

    // Wait to listen on all interfaces.
    block_on(async {
        let mut delay = futures_timer::Delay::new(std::time::Duration::from_secs(1)).fuse();
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
            let mut delay = futures_timer::Delay::new(std::time::Duration::from_secs(10)).fuse();
            loop {
                futures::select! {
                    event = swarm.next() => {
                        match event.unwrap() {
                            SwarmEvent::NewListenAddr { .. } => {}
                            SwarmEvent::Dialing{peer_id,connection_id} => {
                                tracing::info!("Dialing: {:?}, via :{:?}", peer_id, connection_id)
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
                            SwarmEvent::Behaviour(Event::Identify(identify::Event::Sent {
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
                                        rendezvous::Namespace::from_static(lib::NAMESPACE),
                                        relay_peer_id.unwrap(),
                                        None,
                                    ) {
                                    tracing::error!("Failed to register rendezvous point: {error}");
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
                     lib::handle_input_line(&mut swarm.behaviour_mut().kademlia,line.to_string());
                },
                event = swarm.select_next_some() => match event{
                    SwarmEvent::NewListenAddr { address, .. } => {
                        tracing::info!("Listening on {:?}", address);
                    }
                    SwarmEvent::ConnectionEstablished {
                        peer_id, endpoint, ..
                        } => {
                            tracing::info!("Established connection to {:?} via {:?}", peer_id, endpoint);
                            swarm.behaviour_mut().kademlia.add_address(&peer_id,endpoint.get_remote_address().clone());
                            let peers = swarm.connected_peers();
                            for p in peers {
                                tracing::info!("Connected peer: {}",p);
                            };
                        },
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        tracing::info!("Disconnect {:?}", peer_id);
                    },
                    SwarmEvent::OutgoingConnectionError{error,..} => {
                        tracing::debug!("OutgoingConnectionError {:?}", error);
                        // if let Some(peer_id) = peer_id{
                        //     tracing::info!("kad remove peer: {:?}", peer_id);
                        //     swarm.behaviour_mut().kademlia.remove_peer(&peer_id);
                        // }
                    },
                    //Identify events
                    SwarmEvent::Behaviour(Event::Identify(event)) => {
                        lib::identify_event_handler(&mut swarm.behaviour_mut().kademlia, event);
                    },
                    //RendezvousClient events
                    SwarmEvent::Behaviour(Event::Rendezvous(event)) => {
                        lib::rendezvous_client_event_handler(&mut swarm,&mut client_paras, event);
                    },

                    //kad events
                    SwarmEvent::Behaviour(Event::Kademlia(event)) => {
                         lib::kad_event_handler(event);
                    },

                    _ => {
                        tracing::debug!("Event: {:?}", event);
                    }
                },
            }
        }
    });

    Ok(())
}

pub struct ClientParas {
    pub cookie: Option<Cookie>,
    pub rendezvous_point: Option<PeerId>,
    pub bootstrap_node_addr: Option<Multiaddr>,
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Event")]
pub struct Behaviour {
    pub relay_client: relay::client::Behaviour,
    pub identify: identify::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub rendezvous: rendezvous::client::Behaviour,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Event {
    Identify(identify::Event),
    RelayClient(relay::client::Event),
    Dcutr(dcutr::Event),
    Kademlia(KademliaEvent),
    Rendezvous(rendezvous::client::Event),
}

impl From<identify::Event> for Event {
    fn from(e: identify::Event) -> Self {
        Event::Identify(e)
    }
}

impl From<relay::client::Event> for Event {
    fn from(e: relay::client::Event) -> Self {
        Event::RelayClient(e)
    }
}

impl From<dcutr::Event> for Event {
    fn from(e: dcutr::Event) -> Self {
        Event::Dcutr(e)
    }
}

impl From<KademliaEvent> for Event {
    fn from(e: KademliaEvent) -> Self {
        Event::Kademlia(e)
    }
}

impl From<rendezvous::client::Event> for Event {
    fn from(e: rendezvous::client::Event) -> Self {
        Event::Rendezvous(e)
    }
}
