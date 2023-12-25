use std::str::FromStr;
use std::sync::Arc;

use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Quorum, Record};
use libp2p::{kad, PeerId, Swarm};
use tokio::sync::Mutex;

use crate::network::behaviour;
use crate::node::command_handler::CmdHandler;
use crate::node::ClientParas;

pub async fn handle_input_command(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
    line: String,
) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    let mut args = line.split_whitespace();
    match args.next() {
        Some("kad") => {
            let mut swarm = swarm.lock().await;
            handle_kad_command(&mut swarm.behaviour_mut().kademlia, args.collect());
        }
        Some("mega") => {
            handle_mega_command(swarm, client_paras, args.collect()).await;
        }
        Some("nostr") => {
            handle_nostr_command(swarm, client_paras, args.collect()).await;
        }
        _ => {
            eprintln!("expected command: kad, mega, nostr");
        }
    }
}

pub fn handle_kad_command(kademlia: &mut kad::Behaviour<MemoryStore>, args: Vec<&str>) {
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
            let peer_id = match parse_peer_id(args_iter.next()) {
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

pub async fn handle_mega_command(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
    args: Vec<&str>,
) {
    let cmd_handler = CmdHandler {
        swarm,
        client_paras,
    };
    let mut args_iter = args.iter().copied();
    match args_iter.next() {
        //mega provide ${your_repo}.git
        Some("provide") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.provide(&repo_name).await;
        }
        Some("search") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.search(&repo_name).await;
        }
        Some("clone") => {
            // mega clone p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git
            let mega_address = {
                match args_iter.next() {
                    Some(key) => key,
                    None => {
                        eprintln!("Expected mega_address");
                        return;
                    }
                }
            };
            cmd_handler.clone(mega_address).await;
        }
        Some("pull") => {
            // mega pull p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git
            let mega_address = {
                match args_iter.next() {
                    Some(key) => key,
                    None => {
                        eprintln!("Expected mega_address");
                        return;
                    }
                }
            };
            cmd_handler.pull(mega_address).await;
        }
        Some("clone-object") => {
            // mega clone-object mega_test.git
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.clone_obj(&repo_name).await;
        }
        Some("pull-object") => {
            // mega pull-object mega_test.git
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.pull_obj(&repo_name).await;
        }
        _ => {
            eprintln!("expected command: clone, pull, provide, clone-object, pull-object");
        }
    }
}

pub async fn handle_nostr_command(
    swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    client_paras: Arc<Mutex<ClientParas>>,
    args: Vec<&str>,
) {
    let cmd_handler = CmdHandler {
        swarm,
        client_paras,
    };
    let mut args_iter = args.iter().copied();
    match args_iter.next() {
        Some("subscribe") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.subscribe(&repo_name).await;
        }
        Some("event-update") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.event_update(&repo_name).await;
        }
        Some("event-merge") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.event_merge(&repo_name).await;
        }
        Some("event-issue") => {
            let repo_name = {
                match args_iter.next() {
                    Some(path) => path.to_string(),
                    None => {
                        eprintln!("Expected repo_name");
                        return;
                    }
                }
            };
            cmd_handler.event_issue(&repo_name).await;
        }
        _ => {
            eprintln!("expected command: subscribe, event-update, event-issue");
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

pub fn parse_mega_address(mega_address: &str) -> Result<(PeerId, &str), String> {
    // p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git
    let v: Vec<&str> = mega_address.split('/').collect();
    if v.len() < 4 {
        return Err("mega_address invalid".to_string());
    };
    let peer_id = match PeerId::from_str(v[2]) {
        Ok(peer_id) => peer_id,
        Err(e) => return Err(e.to_string()),
    };
    Ok((peer_id, v[3]))
}
