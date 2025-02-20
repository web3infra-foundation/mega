use callisto::git_repo;
use jupiter::context::Context;
use std::{
    net::TcpListener,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn get_short_peer_id(peer_id: String) -> String {
    if peer_id.len() <= 7 {
        return peer_id;
    }
    peer_id[0..7].to_string()
}

pub fn get_available_port() -> Result<u16, String> {
    // Bind to port 0 to let the OS assign an available port
    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();
            Ok(port)
        }
        Err(e) => Err(format!("Failed to bind to a port: {}", e)),
    }
}

pub fn get_utc_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub async fn handle_response(
    request_result: Result<reqwest::Response, reqwest::Error>,
) -> Result<String, String> {
    match request_result {
        Ok(res) => {
            if res.status().is_success() {
                Ok(res.text().await.unwrap())
            } else {
                Err(res.text().await.unwrap())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub async fn repo_alias_to_identifier(alias: String) -> String {
    let (peer_id, _) = vault::init().await;
    format!("p2p://{}/{alias}", peer_id.clone())
}

pub async fn repo_path_to_identifier(http_port: u16, repo_path: String) -> String {
    let (peer_id, _) = vault::init().await;
    format!("p2p://{}/{http_port}{repo_path}.git", peer_id.clone())
}

pub async fn get_ztm_app_tunnel_bound_name(remote_peer_id: String) -> String {
    format!(
        "{}_{}",
        get_short_peer_id(vault::get_peerid().await),
        get_short_peer_id(remote_peer_id)
    )
}

pub async fn get_git_model_by_path(context: Context, path: String) -> Option<git_repo::Model> {
    let git_model = context
        .services
        .git_db_storage
        .find_git_repo_exact_match(path.as_str())
        .await;

    git_model.unwrap_or_default()
}
