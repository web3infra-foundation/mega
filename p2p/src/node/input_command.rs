use crate::network::behaviour;
use crate::node::client::ClientParas;
use libp2p::kad::record::Key;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Kademlia, Quorum, Record};
use libp2p::{PeerId, Swarm};
use std::str::{FromStr, SplitWhitespace};

pub fn handle_input_command(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    line: String,
) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    let mut args = line.split_whitespace();
    match args.next() {
        Some("kad") => {
            handle_kad_command(&mut swarm.behaviour_mut().kademlia, args);
        }
        Some("file") => {
            handle_file_command(swarm, client_paras, args);
        }
        _ => {
            eprintln!("expected command: kad, file");
        }
    }
}

pub fn handle_kad_command(kademlia: &mut Kademlia<MemoryStore>, mut args: SplitWhitespace) {
    match args.next() {
        Some("get") => {
            let key = {
                match args.next() {
                    Some(key) => Key::new(&key),
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
                match args.next() {
                    Some(key) => Key::new(&key),
                    None => {
                        eprintln!("Expected key");
                        return;
                    }
                }
            };
            let value = {
                match args.next() {
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
        Some("k_buckets") => {
            for (_, k_bucket_ref) in kademlia.kbuckets().enumerate() {
                println!("k_bucket_ref.num_entries:{}", k_bucket_ref.num_entries());
                for (_, x) in k_bucket_ref.iter().enumerate() {
                    println!(
                        "PEERS[{:?}]={:?}",
                        x.node.key.preimage().to_string(),
                        x.node.value
                    );
                }
            }
        }
        Some("get_peer") => {
            let peer_id = match parse_peer_id(args.next()) {
                Some(peer_id) => peer_id,
                None => {
                    return;
                }
            };
            kademlia.get_closest_peers(peer_id);
        }
        _ => {
            eprintln!("expected command: get, put, k_buckets, get_peer");
        }
    }
}

pub fn handle_file_command(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    mut args: SplitWhitespace,
) {
    match args.next() {
        Some("get") => {
            let file_name = {
                match args.next() {
                    Some(key) => key,
                    None => {
                        eprintln!("Expected file_name");
                        return;
                    }
                }
            };
            let query_id = swarm
                .behaviour_mut()
                .kademlia
                .get_providers(Key::new(&file_name));
            client_paras
                .pending_request_file
                .insert(query_id, file_name.to_string());
        }
        Some("get_providers") => {
            let key = {
                match args.next() {
                    Some(key) => key.to_string(),
                    None => {
                        eprintln!("Expected key");
                        return;
                    }
                }
            };
            swarm.behaviour_mut().kademlia.get_providers(Key::new(&key));
        }
        Some("provide") => {
            let key = {
                match args.next() {
                    Some(key) => key.to_string(),
                    None => {
                        eprintln!("Expected key");
                        return;
                    }
                }
            };
            let path = {
                match args.next() {
                    Some(value) => value.to_string(),
                    None => {
                        eprintln!("Expected file path");
                        return;
                    }
                }
            };

            if let Err(e) = swarm
                .behaviour_mut()
                .kademlia
                .start_providing(Key::new(&key))
            {
                eprintln!("provide key failed:{:?}", e);
                return;
            };
            client_paras.file_provide_map.insert(key, path);
        }
        _ => {
            eprintln!("expected command: get, provide, get_providers");
        }
    }
}

fn parse_peer_id(peer_id_str: Option<&str>) -> Option<PeerId> {
    match peer_id_str {
        Some(peer_id) => match PeerId::from_str(peer_id) {
            Ok(id) => Some(id),
            Err(err) => {
                eprintln!("peer_id parse error:{}", err);
                None
            }
        },
        None => {
            eprintln!("Expected peer_id");
            None
        }
    }
}
