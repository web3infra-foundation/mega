use std::collections::HashSet;

use reqwest::{get, Client};

use crate::{
    util::handle_response, ztm::get_or_create_remote_mega_tunnel, LFSInfo, LFSInfoPostBody,
    LFSInfoRes,
};

/// share lfs
///
/// ## paras
/// - `bootstrap_node`: bootstrap_node
/// - `file_hash`: file_hash
/// - `hash_type`: hash_type  
/// - `file_size`: file_size  
/// - `origin`: origin  
///
/// ## Example
/// Here is an example of the JSON payload:
/// ```json
/// {
///  "bootstrap_node":"https://gitmono.org/relay",
///  "file_hash":"52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6",
///  "hash_type":"sha256",
///  "file_size":199246498,
///  "origin":"p2p://t14id7uQxwneJ2PnPtaA3GSUwxTx6HTaq1UkayQVWSPT/sha256/52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6"
///}
/// ```
/// This method will send a Post request to the relay to share lfs
///
pub async fn share_lfs(
    bootstrap_node: String,
    file_hash: String,
    hash_type: String,
    file_size: i64,
    origin: String,
) {
    let lfs = LFSInfoPostBody {
        file_hash,
        hash_type,
        file_size,
        peer_id: vault::get_peerid().await,
        origin,
    };
    tracing::info!("Share lfs {:?}", lfs);
    let json = serde_json::to_string(&lfs).unwrap();

    let client = Client::new();
    let url = format!("{}/api/v1/lfs_share", bootstrap_node);
    let response = client
        .post(url)
        .header("content-type", "application/json")
        .body(json)
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        tracing::info!("Share lfs {} successfully!", lfs.file_hash);
    } else {
        let context = response.text().await.unwrap();
        tracing::error!("Share lfs {} failed,{}", lfs.file_hash, context);
    }
}

/// get lfs chunks info
///
/// ## paras
/// - `bootstrap_node`: bootstrap_node
/// - `file_hash`: file_hash
///
/// for example
/// ```json
/// {
///  "bootstrap_node":"https://gitmono.org/relay",
///  "file_hash":"52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6",
///}
/// ```
/// This method will send a GET request to the relay to get lfs chunks info
///
pub async fn get_lfs_chunks_info(bootstrap_node: String, file_hash: String) -> Option<LFSInfoRes> {
    let url = format!(
        "{}/api/v1/lfs_chunk?file_hash={}",
        bootstrap_node, file_hash
    );
    let lfs_info: LFSInfoRes = match get(url.clone()).await {
        Ok(response) => {
            if !response.status().is_success() {
                println!("Get lfs chuncks info failed  {}", url);
                return None;
            }
            let body = response.text().await.unwrap();
            let lfs_info: LFSInfoRes = serde_json::from_str(&body).unwrap();
            lfs_info
        }
        Err(_) => {
            println!("Get lfs chuncks info failed {}", url);
            return None;
        }
    };
    Some(lfs_info)
}

/// create lfs download local ports
///
/// ## Paras
/// - `bootstrap_node`: bootstrap_node
/// - `ztm_agent_port`: ztm_agent_port
/// - `file_uri`: file_uri  
///
/// for example
/// ```json
/// {
///  "bootstrap_node":"https://gitmono.org/relay",
///  "ztm_agent_port":777,
///  "file_uri":"p2p://t14id7uQxwneJ2PnPtaA3GSUwxTx6HTaq1UkayQVWSPT/sha256/52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6"
///}
/// ```
/// ## Return
/// local_port1, local_port2,...
///
/// Each port is for a remote peer
pub async fn create_lfs_download_tunnel(
    bootstrap_node: String,
    ztm_agent_port: u16,
    file_uri: String,
) -> Result<Vec<u16>, String> {
    let file_hash = match get_file_hash_from_origin(file_uri) {
        Ok(file_hash) => file_hash,
        Err(_) => {
            return Err("invalid file_uri".to_string());
        }
    };
    // get public lfs by bootstrap_node
    let url = format!("{bootstrap_node}/api/v1/lfs_list");
    let request_result = reqwest::get(url.clone()).await;
    let response_text = match handle_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            tracing::error!("GET {url} failed,{s}");
            return Err(s);
        }
    };
    let lfs_list: Vec<LFSInfo> = match serde_json::from_slice(response_text.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("{}", e);
            return Err(e.to_string());
        }
    };
    let peer_id = vault::get_peerid().await;
    let peer_list: HashSet<String> = lfs_list
        .iter()
        .filter(|x| x.file_hash == file_hash && x.peer_online && x.peer_id != peer_id)
        .map(|x| x.peer_id.clone())
        .collect();
    tracing::info!("Search lfs[{}] download peer:{:?}", file_hash, peer_list);

    let mut tunnel_list: Vec<u16> = vec![];
    for peer_id in peer_list {
        match get_or_create_remote_mega_tunnel(ztm_agent_port, peer_id).await {
            Ok(port) => {
                tunnel_list.push(port);
            }
            Err(s) => {
                tracing::error!("{}", s);
            }
        }
    }
    Ok(tunnel_list)
}

pub fn get_file_hash_from_origin(origin: String) -> Result<String, String> {
    // p2p://t14id7uQxwneJ2PnPtaA3GSUwxTx6HTaq1UkayQVWSPT/sha256/52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6
    let words: Vec<&str> = origin.split('/').collect();
    if words.len() <= 4 {
        return Err("invalid origin".to_string());
    }
    Ok(words.get(4).unwrap().to_string())
}

#[cfg(test)]
mod tests {
    // use reqwest::get;
    // use std::path::PathBuf;
    // use tokio::fs::OpenOptions;

    // use crate::lfs::create_lfs_download_tunnel;
    // use ceres::lfs::lfs_structs::FetchchunkResponse;
    // use ring::digest::SHA256;
    // use std::fs::{self, create_dir_all};
    // use std::{fs::File, io::Read};
    // use tokio::io::AsyncWriteExt;
    // #[tokio::test]
    // async fn create_lfs_download_tunnel_test() {
    //     let local_port_list = create_lfs_download_tunnel("http://222.20.126.106:8001".to_string()
    //     , 7777,
    //     "p2p://t14id7uQxwneJ2PnPtaA3GSUwxTx6HTaq1UkayQVWSPT/sha256/52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6".to_string()
    // ).await.unwrap();
    //     println!("{:?}", local_port_list);
    //     if local_port_list.len() == 0 {
    //         return;
    //     }

    //     let file_hash = "52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6";
    //     let local_port = local_port_list[0];
    //     println!("Prepare to download LFS {} by chunks", file_hash);
    //     // fetch chunks info http://localhost:{localport}/objects/{object_id}/chunks
    //     let url = format!(
    //         "http://localhost:{}/objects/{}/chunks",
    //         local_port, file_hash
    //     );
    //     let chunk_info: FetchchunkResponse = match get(url.clone()).await {
    //         Ok(response) => {
    //             if !response.status().is_success() {
    //                 println!("Get lfs chuncks info failed  {}", url);
    //                 return;
    //             }
    //             let body = response.text().await.unwrap();
    //             let chunck_info: FetchchunkResponse = serde_json::from_str(&body).unwrap();
    //             chunck_info
    //         }
    //         Err(_) => {
    //             println!("Get lfs chuncks info failed {}", url);
    //             return;
    //         }
    //     };
    //     let mut chunks = chunk_info.chunks;
    //     chunks.sort_unstable_by(|a, b| a.offset.cmp(&b.offset));

    //     // create temp file
    //     let base_dir = PathBuf::from("tmp");
    //     if !base_dir.exists() {
    //         create_dir_all(base_dir.clone()).unwrap();
    //     }
    //     let target_path = base_dir.join(file_hash);
    //     if target_path.exists() {
    //         fs::remove_file(&target_path).unwrap();
    //     }

    //     let mut file = OpenOptions::new()
    //         .create(true)
    //         .append(true)
    //         .open(target_path.clone())
    //         .await
    //         .unwrap();

    //     println!(
    //         "Start to download LFS chunks,size: {}B, chunk_num: {}",
    //         chunk_info.size,
    //         chunks.len()
    //     );
    //     for (index, chunk) in chunks.iter().enumerate() {
    //         // http://localhost:{localport}/objects/{object_id}/chunks
    //         let local_port = local_port_list[index % local_port_list.len()];
    //         let url = format!("http://localhost:{}/objects/{}", local_port, chunk.sub_oid);
    //         let data = match get(url.clone()).await {
    //             Ok(response) => {
    //                 if !response.status().is_success() {
    //                     println!("Get lfs chuncks failed  {}", url);
    //                     return;
    //                 }
    //                 response.bytes().await.unwrap()
    //             }
    //             Err(_) => {
    //                 println!("Get lfs chuncks failed {}", url);
    //                 return;
    //             }
    //         };
    //         file.write_all(&data).await.unwrap();
    //         println!("Chunk[{}] download from {} successfully", index, url);
    //     }

    //     println!("Download LFS {} by chunks successfully", file_hash);

    //     let mut file = File::open(target_path).unwrap();

    //     let mut context = ring::digest::Context::new(&SHA256);
    //     let mut buffer = [0u8; 1024];

    //     loop {
    //         let count = file.read(&mut buffer).unwrap();
    //         if count == 0 {
    //             break;
    //         }
    //         context.update(&buffer[..count]);
    //     }

    //     let checksum = hex::encode(context.finish().as_ref());
    //     let result = checksum == file_hash;
    //     if result {
    //         println!("Check LFS SHA256({}) successfully", file_hash);
    //     } else {
    //         println!("Check LFS SHA256 failed");
    //     }
    // }

    // #[tokio::test]
    // async fn test_get_chunk_info() {
    //     let res = super::get_lfs_chunks_info(
    //         "http://localhost:8001".to_string(),
    //         "52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6".to_string(),
    //     )
    //     .await;
    //     println!("{:?}", res);
    // }
}
