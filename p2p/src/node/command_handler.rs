use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_std::sync::RwLock;
use common::utils;
use libp2p::kad::{self, Quorum, Record};
use libp2p::Swarm;
use secp256k1::{Secp256k1, rand, KeyPair};

use crate::get_utc_timestamp;
use crate::network::behaviour::{GitInfoRefsReq, GitUploadPackReq};
use crate::node::MegaRepoInfo;
use crate::nostr::NostrReq;
use crate::nostr::client_message::{ClientMessage, SubscriptionId, Filter};
use crate::nostr::event::{GitEvent, NostrEvent};
use crate::{get_pack_protocol, get_repo_full_path, network::behaviour, node::ClientParas};

use super::input_command::parse_mega_address;

pub struct CmdHandler {
    pub swarm: Arc<Mutex<Swarm<behaviour::Behaviour>>>,
    pub client_paras: Arc<RwLock<ClientParas>>,
}

impl CmdHandler {
    pub async fn provide(&self, repo_name: &str) {
        let client_paras = self.client_paras.read().await;
        if !repo_name.ends_with(".git") {
            eprintln!("repo_name should end with .git");
            return;
        }
        let path = get_repo_full_path(repo_name);
        let pack_protocol: git::protocol::PackProtocol =
            get_pack_protocol(&path, client_paras.storage.clone());

        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
        if object_id == *utils::ZERO_ID {
            eprintln!("Repository not found");
            return;
        }

        let mut swarm = self.swarm.lock().unwrap();
        // //Construct repoInfo
        let mega_repo_info = MegaRepoInfo {
            origin: swarm.local_peer_id().to_string(),
            name: repo_name.to_string(),
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

    pub async fn search(&self, repo_name: &str) {
        let mut swarm = self.swarm.lock().unwrap();
        swarm
            .behaviour_mut()
            .kademlia
            .get_record(kad::RecordKey::new(&repo_name));
    }

    pub async fn clone(&self, mega_address: &str) {
        // mega clone p2p://12D3KooWFgpUQa9WnTztcvs5LLMJmwsMoGZcrTHdt9LKYKpM4MiK/abc.git
        let mut swarm = self.swarm.lock().unwrap();

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
        {
            let mut client_paras = self.client_paras.write().await;
            client_paras
                .pending_git_upload_package
                .insert(request_file_id, repo_name.to_string());
        }
    }

    pub async fn pull(&self, mega_address: &str) {
        let (peer_id, repo_name) = match parse_mega_address(mega_address) {
            Ok((peer_id, repo_name)) => (peer_id, repo_name),
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        let path = get_repo_full_path(repo_name);
        let mut client_paras = self.client_paras.write().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
        if object_id == *utils::ZERO_ID {
            eprintln!("local repo not found");
            return;
        }
        {
            // Request to get git_info_refs
            let mut swarm = self.swarm.lock().unwrap();
            let request_id = swarm
                .behaviour_mut()
                .git_info_refs
                .send_request(&peer_id, GitInfoRefsReq(path, Vec::new()));
            client_paras
                .pending_git_pull
                .insert(request_id, repo_name.to_string());
        }
    }

    pub async fn clone_obj(&self, repo_name: &str) {
        if !repo_name.ends_with(".git") {
            eprintln!("repo_name should end with .git");
            return;
        }

        let mut swarm = self.swarm.lock().unwrap();
        let kad_query_id = swarm
            .behaviour_mut()
            .kademlia
            .get_record(kad::RecordKey::new(&repo_name));

        {
            let mut client_paras = self.client_paras.write().await;
            client_paras
                .pending_repo_info_search_to_download_obj
                .insert(kad_query_id, repo_name.to_owned());
        }
    }

    pub async fn pull_obj(&self, repo_name: &str) {
        if !repo_name.ends_with(".git") {
            eprintln!("repo_name should end with .git");
            return;
        }
        let mut swarm = self.swarm.lock().unwrap();
        let kad_query_id = swarm
            .behaviour_mut()
            .kademlia
            .get_record(kad::RecordKey::new(&repo_name));
        {
            let mut client_paras = self.client_paras.write().await;

            client_paras
                .pending_repo_info_search_to_download_obj
                .insert(kad_query_id, repo_name.to_owned());
        }
    }


    pub async fn subscribe(&self, repo_name: &str) {
        let client_paras = self.client_paras.read().await;
        let relay_peer_id = client_paras.rendezvous_point.unwrap();
        let filters = vec![Filter::new().repo_name(repo_name.to_string())];
        let client_req = ClientMessage::new_req(SubscriptionId::generate(), filters);

        {
            let mut swarm = self.swarm.lock().unwrap();
            swarm
                .behaviour_mut()
                .nostr
                .send_request(&relay_peer_id, NostrReq(client_req.as_json()));
        }

    }

    pub async fn event_update(&self, repo_name: &str) {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);

        let mut swarm = self.swarm.lock().unwrap();

        let peer_id = swarm.local_peer_id().to_string();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name);
        let client_paras = self.client_paras.read().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id,
            repo_name: repo_name.to_string(),
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

    pub async fn event_merge(&self, repo_name: &str) {
        let mut swarm = self.swarm.lock().unwrap();

        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let peer_id = swarm.local_peer_id().to_string();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name);
        let client_paras = self.client_paras.read().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id: swarm.local_peer_id().to_string(),
            repo_name: repo_name.to_string(),
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

    pub async fn event_issue(&self, repo_name: &str) {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let swarm = &mut self.swarm.lock().unwrap();
        let peer_id = swarm.local_peer_id().to_string();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name);
        let client_paras = self.client_paras.read().await;
        let pack_protocol = get_pack_protocol(&path, client_paras.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id: swarm.local_peer_id().to_string(),
            repo_name: repo_name.to_string(),
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
}
