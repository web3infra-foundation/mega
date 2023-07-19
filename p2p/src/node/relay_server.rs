use super::lib;
use async_std::io;
use async_std::io::prelude::BufReadExt;
use futures::executor::block_on;
use futures::stream::StreamExt;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::Kademlia;
use libp2p::{
    core::upgrade,
    core::Transport,
    identify, identity,
    identity::PeerId,
    noise, relay, rendezvous,
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp,
};
use std::error::Error;

pub fn run(local_key: identity::Keypair, p2p_address: String) -> Result<(), Box<dyn Error>> {
    let local_peer_id = PeerId::from(local_key.public());
    tracing::info!("Local peer id: {local_peer_id:?}");

    let tcp_transport = tcp::async_io::Transport::default();

    let tcp_transport = tcp_transport
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(
            noise::Config::new(&local_key).expect("Signing libp2p-noise static DH keypair failed."),
        )
        .multiplex(libp2p::yamux::Config::default())
        .boxed();

    let store = MemoryStore::new(local_peer_id);

    let behaviour = ServerBehaviour {
        relay: relay::Behaviour::new(local_peer_id, Default::default()),
        identify: identify::Behaviour::new(identify::Config::new(
            "/mega/0.0.1".to_string(),
            local_key.public(),
        )),
        kademlia: Kademlia::new(local_peer_id, store),
        rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
    };

    let mut swarm = SwarmBuilder::without_executor(tcp_transport, behaviour, local_peer_id).build();

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
                     lib::handle_input_line(&mut swarm.behaviour_mut().kademlia,line.to_string());
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
                         lib::kad_event_handler(event);
                    }
                    //RendezvousServer events
                    SwarmEvent::Behaviour(ServerBehaviourEvent::Rendezvous(event)) => {
                        lib::rendezvous_server_event_handler(event);
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

#[derive(NetworkBehaviour)]
pub struct ServerBehaviour {
    pub relay: relay::Behaviour,
    pub identify: identify::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub rendezvous: rendezvous::server::Behaviour,
}
