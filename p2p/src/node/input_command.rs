use super::ClientParas;
use crate::network::behaviour;
use crate::network::behaviour::{GitInfoRefsReq, GitUploadPackReq};
use crate::node::{MegaRepoInfo};
use crate::{get_pack_protocol, get_repo_full_path, get_utc_timestamp};
use common::utils;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Quorum, Record};
use libp2p::{kad, PeerId, Swarm};
use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;
use secp256k1::{KeyPair, rand, Secp256k1};
use crate::nostr::client_message::{ClientMessage, Filter, SubscriptionId};
use crate::nostr::event::{GitEvent, NostrEvent};
use crate::nostr::NostrReq;

pub async fn handle_input_command(
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
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    args: Vec<&str>,
) {
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
            if !repo_name.ends_with(".git") {
                eprintln!("repo_name should end with .git");
                return;
            }
            let path = get_repo_full_path(&repo_name);
            let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
            let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
            if object_id == *utils::ZERO_ID {
                eprintln!("Repository not found");
                return;
            }
            //Construct repoInfo
            let mega_repo_info = MegaRepoInfo {
                origin: swarm.local_peer_id().to_string(),
                name: repo_name.clone(),
                latest: object_id,
                forks: vec![],
                timestamp: get_utc_timestamp(),
            };

            let record = Record {
                key: kad::RecordKey::new(&repo_name),
                value: serde_json::to_vec(&mega_repo_info).unwrap(),
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
            swarm
                .behaviour_mut()
                .kademlia
                .get_record(kad::RecordKey::new(&repo_name));
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
            let (peer_id, repo_name) = match parse_mega_address(mega_address) {
                Ok((peer_id, repo_name)) => (peer_id, repo_name),
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };
            let path = get_repo_full_path(repo_name);
            let request_file_id = swarm.behaviour_mut().git_upload_pack.send_request(
                &peer_id,
                GitUploadPackReq(HashSet::new(), HashSet::new(), path),
            );
            client_paras
                .pending_git_upload_package
                .insert(request_file_id, repo_name.to_string());
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
            let (peer_id, repo_name) = match parse_mega_address(mega_address) {
                Ok((peer_id, repo_name)) => (peer_id, repo_name),
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };
            let path = get_repo_full_path(repo_name);
            let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
            let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
            if object_id == *utils::ZERO_ID {
                eprintln!("local repo not found");
                return;
            }
            // Request to get git_info_refs
            let request_id = swarm
                .behaviour_mut()
                .git_info_refs
                .send_request(&peer_id, GitInfoRefsReq(path, Vec::new()));
            client_paras
                .pending_git_pull
                .insert(request_id, repo_name.to_string());
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
            if !repo_name.ends_with(".git") {
                eprintln!("repo_name should end with .git");
                return;
            }

            let kad_query_id = swarm
                .behaviour_mut()
                .kademlia
                .get_record(kad::RecordKey::new(&repo_name));
            client_paras
                .pending_repo_info_search_to_download_obj
                .insert(kad_query_id, repo_name);
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
            if !repo_name.ends_with(".git") {
                eprintln!("repo_name should end with .git");
                return;
            }

            let kad_query_id = swarm
                .behaviour_mut()
                .kademlia
                .get_record(kad::RecordKey::new(&repo_name));
            client_paras
                .pending_repo_info_search_to_download_obj
                .insert(kad_query_id, repo_name);
        }
        _ => {
            eprintln!("expected command: clone, pull, provide, clone-object, pull-object");
        }
    }
}


pub async fn handle_nostr_command(
    swarm: &mut Swarm<behaviour::Behaviour>,
    client_paras: &mut ClientParas,
    args: Vec<&str>,
) {
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

            let relay_peer_id = client_paras.rendezvous_point.unwrap();
            let filters = vec![
                Filter::new().repo_name(repo_name),
            ];
            let client_req = ClientMessage::new_req(SubscriptionId::generate(), filters);
            swarm
                .behaviour_mut()
                .nostr
                .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
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

            let secp = Secp256k1::new();
            let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
            let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
            let peer_id = swarm.local_peer_id().to_string();
            let url = format!("p2p://{}/{}", peer_id, repo_name);

            let path = get_repo_full_path(repo_name.as_str());
            let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
            let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

            let git_event = GitEvent {
                peer_id: swarm.local_peer_id().to_string(),
                repo_name,
                repo_target: "origin".to_string(),
                repo_action: "update".to_string(),
                repo_url: url,
                repo_commit_id: object_id,
                repo_issue_content: "".to_string(),
            };
            let event = NostrEvent::new_git_event(key_pair, git_event);

            let client_req = ClientMessage::new_event(event);
            let relay_peer_id = client_paras.rendezvous_point.unwrap();
            swarm
                .behaviour_mut()
                .nostr
                .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
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

            let secp = Secp256k1::new();
            let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
            let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
            let peer_id = swarm.local_peer_id().to_string();
            let url = format!("p2p://{}/{}", peer_id, repo_name);

            let path = get_repo_full_path(repo_name.as_str());
            let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
            let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

            let git_event = GitEvent {
                peer_id: swarm.local_peer_id().to_string(),
                repo_name,
                repo_target: "fork".to_string(),
                repo_action: "request".to_string(),
                repo_url: url,
                repo_commit_id: object_id,
                repo_issue_content: "".to_string(),
            };
            let event = NostrEvent::new_git_event(key_pair, git_event);

            let client_req = ClientMessage::new_event(event);
            let relay_peer_id = client_paras.rendezvous_point.unwrap();
            swarm
                .behaviour_mut()
                .nostr
                .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
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

            let secp = Secp256k1::new();
            let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
            let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
            let peer_id = swarm.local_peer_id().to_string();
            let url = format!("p2p://{}/{}", peer_id, repo_name);

            let path = get_repo_full_path(repo_name.as_str());
            let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone()).await;
            let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

            let git_event = GitEvent {
                peer_id: swarm.local_peer_id().to_string(),
                repo_name,
                repo_target: "fork".to_string(),
                repo_action: "issue".to_string(),
                repo_url: url,
                repo_commit_id: object_id,
                repo_issue_content: "new issue".to_string(),
            };
            let event = NostrEvent::new_git_event(key_pair, git_event);

            let client_req = ClientMessage::new_event(event);
            let relay_peer_id = client_paras.rendezvous_point.unwrap();
            swarm
                .behaviour_mut()
                .nostr
                .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
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

fn parse_mega_address(mega_address: &str) -> Result<(PeerId, &str), String> {
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
