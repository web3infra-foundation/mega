use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use bytes::Bytes;
use entity::objects::Model;
use git::protocol::RefCommand;
use git::structure::conversion;
use libp2p::kad::{self, GetRecordOk, Record};
use libp2p::PeerId;
use secp256k1::{rand, KeyPair, Secp256k1};

use common::utils;
use storage::driver::database::storage::ObjectStorage;

use crate::get_utc_timestamp;
use crate::network::behaviour::{GitInfoRefsReq, GitObjectReq, GitUploadPackReq, GitUploadPackRes};
use crate::network::{get_all_git_obj_ids, Client};
use crate::node::{Fork, MegaRepoInfo};
use crate::nostr::client_message::{ClientMessage, Filter, SubscriptionId};
use crate::nostr::event::{GitEvent, NostrEvent};
use crate::nostr::NostrReq;
use crate::{get_pack_protocol, get_repo_full_path};

pub struct HttpHandler {
    pub network_client: Client,
    pub storage: Arc<dyn ObjectStorage>,
    pub local_peer_id: String,
    pub relay_peer_id: String,
}

impl HttpHandler {
    pub async fn mega_provide(
        &mut self,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        if !repo_name.ends_with(".git") {
            eprintln!("repo_name should end with .git");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Repo name should end with .git"),
            ));
        }
        let path = get_repo_full_path(repo_name.as_str());
        let pack_protocol: git::protocol::PackProtocol =
            get_pack_protocol(&path, self.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
        if object_id == *utils::ZERO_ID {
            eprintln!("Repository not found");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Repository not found"),
            ));
        }
        //Construct repoInfo
        let mega_repo_info = MegaRepoInfo {
            origin: self.local_peer_id.clone(),
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

        let result = self.network_client.put_record(record).await;
        match result {
            Ok(ok) => Ok(Json(String::from_utf8(ok.key.to_vec()).unwrap())),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        }
    }

    pub async fn mega_search(
        &mut self,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let key = kad::RecordKey::new(&repo_name);
        let result = self.network_client.get_record(key).await;
        if let Ok(GetRecordOk::FoundRecord(peer_record)) = result {
            Ok(Json(String::from_utf8(peer_record.record.value).unwrap()))
        } else {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("The record was not found"),
            ))
        }
    }

    pub async fn mega_clone(
        &mut self,
        mega_address: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let (peer_id, repo_name) = match parse_mega_address(mega_address.as_str()) {
            Ok((peer_id, repo_name)) => (peer_id, repo_name),
            Err(e) => {
                eprintln!("{}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
            }
        };
        let path = get_repo_full_path(repo_name);

        //request to peer_id for pack
        let git_upload_pack_res = self
            .network_client
            .git_upload_pack(
                peer_id,
                GitUploadPackReq(Vec::new(), Vec::new(), path),
            )
            .await;
        match git_upload_pack_res {
            Ok(res) => {
                //deal the pack data
                self.deal_git_upload_pack_res(res, self.storage.clone(), repo_name.to_string())
                    .await
            }
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        }
    }

    pub async fn mega_pull(
        &mut self,
        mega_address: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let (peer_id, repo_name) = match parse_mega_address(mega_address.as_str()) {
            Ok((peer_id, repo_name)) => (peer_id, repo_name),
            Err(e) => {
                eprintln!("{}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
            }
        };
        let path = get_repo_full_path(repo_name);
        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
        if object_id == *utils::ZERO_ID {
            eprintln!("Local repo not found");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Local repo not found"),
            ));
        }

        // Request to get git_info_refs
        let git_info_refs_res_result = self
            .network_client
            .git_info_refs(peer_id, GitInfoRefsReq(path, Vec::new()))
            .await;

        let git_info_refs_res = match git_info_refs_res_result {
            Ok(res) => res,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };

        //have git_ids and try to send pull request
        let ref_git_id = git_info_refs_res.0;
        let _git_ids = git_info_refs_res.1;
        tracing::info!("repo_name: {}", repo_name);
        tracing::info!("ref_git_id: {:?}", ref_git_id);
        if ref_git_id == *utils::ZERO_ID {
            eprintln!("Remote repo not found");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Remote repo not found"),
            ));
        }
        let path = get_repo_full_path(repo_name);
        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
        //generate want and have collection
        let mut want: Vec<String> = Vec::new();
        let mut have: Vec<String> = Vec::new();
        want.push(ref_git_id);
        let commit_models = pack_protocol
            .storage
            .get_all_commits_by_path(&path)
            .await
            .unwrap();
        commit_models.iter().for_each(|model| {
            have.push(model.git_id.clone());
        });

        //get git pack from peer_id
        let git_upload_pack_res = self
            .network_client
            .git_upload_pack(peer_id, GitUploadPackReq(want, have, path))
            .await;

        match git_upload_pack_res {
            Ok(res) => {
                //deal the pack data
                self.deal_git_upload_pack_res(res, self.storage.clone(), repo_name.to_string())
                    .await
            }
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        }
    }

    pub async fn mega_clone_or_pull_obj(
        &mut self,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        if !repo_name.ends_with(".git") {
            eprintln!("repo_name should end with .git");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("repo_name should end with .git"),
            ));
        }

        //search DHT to get repoInfo
        let kad_result = self
            .network_client
            .get_record(kad::RecordKey::new(&repo_name))
            .await;

        if let Err(e) = kad_result {
            eprintln!("{}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
        let record = match kad_result {
            Ok(GetRecordOk::FoundRecord(record)) => record,
            _ => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    String::from("Record not found"),
                ));
            }
        };
        //try to search origin node
        tracing::info!("try to get origin node to search git_obj_id_list");
        let repo_info: MegaRepoInfo = match serde_json::from_slice(&record.record.value) {
            Ok(p) => p,
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
            }
        };

        //save all node that have this repo,the first one is origin
        let mut node_id_list: Vec<String> = Vec::new();
        node_id_list.push(repo_info.origin.clone());
        for fork in &repo_info.forks {
            node_id_list.push(fork.peer.clone());
        }

        let remote_peer_id = PeerId::from_str(&repo_info.origin).unwrap();
        let path = get_repo_full_path(repo_name.as_str());
        //to get local git_obj id
        let local_git_ids = get_all_git_obj_ids(&path, self.storage.clone()).await;
        let git_info_refs_res_result = self
            .network_client
            .git_info_refs(remote_peer_id, GitInfoRefsReq(path, local_git_ids))
            .await;

        let git_info_refs_res = match git_info_refs_res_result {
            Ok(res) => res,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };

        // have git_ids and try to download git obj
        let _ref_git_id = git_info_refs_res.0;
        let git_ids_need = git_info_refs_res.1;
        let path = get_repo_full_path(repo_name.as_str());
        tracing::info!("path: {}", path);
        tracing::info!("git_ids_need: {:?}", git_ids_need);

        if node_id_list.is_empty() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("No peer available"),
            ));
        }

        //trying to download git_obj from peers
        node_id_list.retain(|r| *r != self.local_peer_id);
        tracing::info!("try to download git object from: {:?}", node_id_list);
        tracing::info!("the origin is: {}", node_id_list[0]);

        // Try to download separately
        //TODO multi thread
        let mut receive_git_obj_model: Vec<Model> = Vec::new();
        let split_git_ids = split_array(git_ids_need.clone(), node_id_list.len());
        for i in 0..node_id_list.len() {
            // send get git object request
            let ids = split_git_ids[i].clone();
            let repo_peer_id = PeerId::from_str(&node_id_list[i].clone()).unwrap();
            let git_object_res_result = self
                .network_client
                .git_object(repo_peer_id, GitObjectReq(path.clone(), ids))
                .await;

            let git_object_res = match git_object_res_result {
                Ok(res) => res,
                Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
            };

            let git_obj_models = git_object_res.0;
            tracing::info!(
                "Receive {:?} git_obj, from {:?}",
                git_obj_models.len(),
                repo_peer_id
            );
            let receive_id_list: Vec<String> = git_obj_models
                .clone()
                .iter()
                .map(|m| m.git_id.clone())
                .collect();
            tracing::info!("git_obj_id_list:{:?}", receive_id_list);
            receive_git_obj_model.append(&mut git_obj_models.clone());
        }

        tracing::info!("receive all git_object :{:?}", receive_git_obj_model.len());
        match conversion::save_node_from_git_obj(
            self.storage.clone(),
            Path::new(&path),
            receive_git_obj_model.clone(),
        )
        .await
        {
            Ok(_) => {
                tracing::info!(
                    "Save {:?} git_obj to database successfully",
                    receive_git_obj_model.len()
                );
            }
            Err(_e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    String::from("Save git_obj to database failed"),
                ));
            }
        }
        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
        //update repoInfo
        self.update_dht_repo_forks(repo_name.clone(), object_id)
            .await?;

        //subscribe
        self.nostr_subscribe(repo_name.clone()).await
    }

    pub async fn nostr_subscribe(
        &mut self,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let relay_peer_id = self.relay_peer_id.clone();
        let filters = vec![Filter::new().repo_name(repo_name.to_string())];
        let client_req = ClientMessage::new_req(SubscriptionId::generate(), filters);
        let remote_peer_id = PeerId::from_str(&relay_peer_id).unwrap();
        let nostr_res_result = self
            .network_client
            .nostr(remote_peer_id, NostrReq(client_req.as_json()))
            .await;

        let nostr_res = match nostr_res_result {
            Ok(res) => res,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };

        Ok(Json(nostr_res.0))
    }

    pub async fn nostr_event_update(
        &mut self,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);

        let peer_id = self.local_peer_id.clone();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name.as_str());
        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
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
        let relay_peer_id = self.relay_peer_id.clone();
        let remote_peer_id = PeerId::from_str(&relay_peer_id).unwrap();
        let nostr_res_result = self
            .network_client
            .nostr(remote_peer_id, NostrReq(client_req.as_json()))
            .await;

        let nostr_res = match nostr_res_result {
            Ok(res) => res,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };

        Ok(Json(nostr_res.0))
    }

    pub async fn nostr_event_merge(
        &mut self,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let peer_id = self.local_peer_id.clone();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name.as_str());
        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id,
            repo_name: repo_name.to_string(),
            repo_target: "fork".to_string(),
            repo_action: "request".to_string(),
            repo_url: url,
            repo_commit_id: object_id,
            repo_issue_content: "".to_string(),
        };
        let event = NostrEvent::new_git_event(key_pair, git_event);

        let client_req = ClientMessage::new_event(event);
        let relay_peer_id = self.relay_peer_id.clone();
        let remote_peer_id = PeerId::from_str(&relay_peer_id).unwrap();
        let nostr_res_result = self
            .network_client
            .nostr(remote_peer_id, NostrReq(client_req.as_json()))
            .await;

        let nostr_res = match nostr_res_result {
            Ok(res) => res,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };

        Ok(Json(nostr_res.0))
    }

    pub async fn nostr_event_issue(
        &mut self,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut rand::thread_rng());
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let peer_id = self.local_peer_id.clone();
        let url = format!("p2p://{}/{}", peer_id, repo_name);

        let path = get_repo_full_path(repo_name.as_str());
        let pack_protocol = get_pack_protocol(&path, self.storage.clone());
        let object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;

        let git_event = GitEvent {
            peer_id,
            repo_name: repo_name.to_string(),
            repo_target: "fork".to_string(),
            repo_action: "issue".to_string(),
            repo_url: url,
            repo_commit_id: object_id,
            repo_issue_content: "new issue".to_string(),
        };
        let event = NostrEvent::new_git_event(key_pair, git_event);

        let client_req = ClientMessage::new_event(event);
        let relay_peer_id = self.relay_peer_id.clone();
        let remote_peer_id = PeerId::from_str(&relay_peer_id).unwrap();
        let nostr_res_result = self
            .network_client
            .nostr(remote_peer_id, NostrReq(client_req.as_json()))
            .await;

        let nostr_res = match nostr_res_result {
            Ok(res) => res,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        };

        Ok(Json(nostr_res.0))
    }

    pub async fn deal_git_upload_pack_res(
        &mut self,
        git_upload_pack_res: GitUploadPackRes,
        storage: Arc<dyn ObjectStorage>,
        repo_name: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        //dealing the pack from peer
        let package_data = git_upload_pack_res.0;
        let object_id = git_upload_pack_res.1;
        if package_data.starts_with("ERR:".as_bytes()) {
            let e = String::from_utf8(package_data).unwrap();
            tracing::error!("{}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
        let path = get_repo_full_path(repo_name.as_str());
        let mut pack_protocol = get_pack_protocol(&path, storage.clone());
        let old_object_id = pack_protocol.get_head_object_id(Path::new(&path)).await;
        tracing::info!(
            "new_object_id:{}; old_object_id:{}",
            object_id.clone(),
            old_object_id
        );
        let command = RefCommand::new(
            old_object_id,
            object_id.clone(),
            String::from("refs/heads/master"),
        );
        pack_protocol.command_list.push(command);
        let result = pack_protocol
            .git_receive_pack(Bytes::from(package_data))
            .await;
        match result {
            Ok(_) => {
                tracing::info!("Save git package successfully :{}", repo_name);
                //update repoInfo
                self.update_dht_repo_forks(repo_name.clone(), object_id)
                    .await?;

                //subscribe
                self.nostr_subscribe(repo_name.clone()).await
            }
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        }
    }

    pub async fn update_dht_repo_forks(
        &mut self,
        repo_name: String,
        object_id: String,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let dht_result = self
            .network_client
            .get_record(kad::RecordKey::new(&repo_name))
            .await;
        let record = match dht_result {
            Ok(GetRecordOk::FoundRecord(record)) => record.record,
            _ => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    String::from("Update dht failed"),
                ));
            }
        };
        tracing::info!("update repo info forks");
        // update repo info forks
        if let Ok(p) = serde_json::from_slice(&record.value) {
            let mut repo_info: MegaRepoInfo = p;
            let local_peer_id = self.local_peer_id.to_string();
            let fork = Fork {
                peer: local_peer_id.clone(),
                latest: object_id.clone(),
                timestamp: get_utc_timestamp(),
            };
            repo_info.forks.retain(|r| r.peer != local_peer_id);
            repo_info.forks.push(fork);
            let record = Record {
                key: kad::RecordKey::new(&repo_info.name),
                value: serde_json::to_vec(&repo_info).unwrap(),
                publisher: None,
                expires: None,
            };
            if let Err(e) = self.network_client.put_record(record).await {
                eprintln!("Failed to store record:{}", e);
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
            }
        }
        Ok(Json(String::from("ok")))
    }
}

pub fn parse_peer_id(peer_id_str: Option<&str>) -> Option<PeerId> {
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

fn split_array(a: Vec<String>, count: usize) -> Vec<Vec<String>> {
    let mut result = vec![];
    let split_num = a.len() / count;
    for i in 0..count {
        let v: Vec<_> = if i != count - 1 {
            a.clone()
                .drain(i * split_num..(i + 1) * split_num)
                .collect()
        } else {
            a.clone().drain(i * split_num..).collect()
        };
        result.push(v);
    }
    result
}
