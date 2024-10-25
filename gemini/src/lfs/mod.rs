use reqwest::Client;

use crate::{
    util::handle_response, ztm::get_or_create_remote_mega_tunnel, LFSInfo, LFSInfoPostBody,
};

pub async fn share_lfs(bootstrap_node: String, lfs: LFSInfoPostBody) {
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

pub async fn create_lfs_download_tunnel(
    bootstrap_node: String,
    ztm_agent_port: u16,
    file_uri: String,
) -> Result<Vec<String>, String> {
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
    let peer_list: Vec<String> = lfs_list
        .iter()
        .filter(|x| x.file_hash == file_hash && x.peer_online && x.peer_id != vault::get_peerid())
        .map(|x| x.peer_id.clone())
        .collect();
    tracing::info!("Search lfs[{}] download peer:{:?}", file_hash, peer_list);

    let mut tunnel_list: Vec<String> = vec![];
    for peer_id in peer_list {
        match get_or_create_remote_mega_tunnel(ztm_agent_port, peer_id).await {
            Ok(port) => {
                tunnel_list.push(format!("http://localhost:{}", port));
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
    return Ok(words.get(4).unwrap().to_string());
}

#[cfg(test)]
mod tests {
    use crate::lfs::create_lfs_download_tunnel;

    #[tokio::test]
    async fn create_lfs_download_tunnel_test() {
        let result = create_lfs_download_tunnel("http://222.20.126.106:8001".to_string()
        , 7777,
        "p2p://t14id7uQxwneJ2PnPtaA3GSUwxTx6HTaq1UkayQVWSPT/sha256/52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6".to_string()
    ).await.unwrap();
        println!("{:?}", result);
    }
}
