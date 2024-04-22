use std::error::Error;

use libp2p::identity::secp256k1::SecretKey;
use libp2p::identity::Keypair;
use libp2p::{identity, multiaddr, Multiaddr, PeerId};
use tokio::join;

use common::enums::DataSource;
use storage::driver::database;

use crate::http::client_http;
use crate::network;

pub async fn run(
    secret_key: secp256k1::SecretKey,
    p2p_address: String,
    bootstrap_node: String,
    data_source: DataSource,
    p2p_http_port: u16,
) -> Result<(), Box<dyn Error>> {
    tracing::info!("Connect to database");
    let storage = database::init(&data_source).await;

    let local_key = get_local_keypair(secret_key);
    let local_peer_id = PeerId::from(local_key.public());
    tracing::info!("Local peer id: {local_peer_id:?}");

    let (mut network_client, network_event_loop) =
        network::new(local_key, storage.clone()).await.unwrap();

    // Spawn the network task for it to run in the background.
    let libp2p_event_task = tokio::spawn(network_event_loop.run());

    network_client
        .start_listening(p2p_address.parse().unwrap())
        .await
        .unwrap();

    //dial to bootstrap_node
    let mut relay_peer_id: PeerId = PeerId::random();
    if !bootstrap_node.is_empty() {
        let bootstrap_node_addr: Multiaddr = bootstrap_node.parse().unwrap();
        relay_peer_id = network_client.dial(bootstrap_node_addr.clone()).await;
        tracing::info!("relay_peer_id:{}", relay_peer_id.to_string());
        let bootstrap_node_addr = get_full_node_addr(relay_peer_id, bootstrap_node_addr);
        tracing::info!("bootstrap_node_addr:{}", bootstrap_node_addr.to_string());

        //rendezvous register
        if let Err(e) = network_client
            .rendezvous_register(relay_peer_id, bootstrap_node_addr)
            .await
        {
            tracing::error!("Rendezvous register err :{}", e);
        }
    }

    //http server
    let p2p_http_task = tokio::spawn(async move {
        client_http::server(
            network_client,
            storage.clone(),
            local_peer_id.to_string(),
            relay_peer_id.to_string(),
            p2p_http_port,
        )
        .await;
    });

    join!(p2p_http_task, libp2p_event_task).0.unwrap();
    Ok(())
}

pub fn get_local_keypair(secret_key: secp256k1::SecretKey) -> Keypair {
    //secp256k1 keypair
    let secp = secp256k1::Secp256k1::new();
    let _key_pair = secp256k1::KeyPair::from_secret_key(&secp, &secret_key);

    //libp2p keypair with same sk
    let libp2p_sk = SecretKey::try_from_bytes(secret_key.secret_bytes()).unwrap();
    let secp256k1_kp = identity::secp256k1::Keypair::from(libp2p_sk.clone());
    identity::Keypair::from(secp256k1_kp)
}

pub fn get_full_node_addr(peer_id: PeerId, node_addr: Multiaddr) -> Multiaddr {
    let p2p_suffix = multiaddr::Protocol::P2p(peer_id);
    if !node_addr.ends_with(&Multiaddr::empty().with(p2p_suffix.clone())) {
        node_addr.clone().with(p2p_suffix)
    } else {
        node_addr.clone()
    }
}
