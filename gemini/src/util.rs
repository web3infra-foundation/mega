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
    let peer_id = vault::get_peerid().await;
    format!("p2p://{}/{alias}", peer_id.clone())
}

pub async fn repo_path_to_identifier(repo_path: String) -> String {
    let peer_id = vault::get_peerid().await;
    if repo_path.starts_with("/") {
        return format!("p2p://{}{repo_path}.git", peer_id.clone());
    }
    format!("p2p://{}/{repo_path}.git", peer_id.clone())
}

pub async fn get_ztm_app_tunnel_bound_name(remote_peer_id: String) -> String {
    format!(
        "{}_{}",
        get_short_peer_id(vault::get_peerid().await),
        get_short_peer_id(remote_peer_id)
    )
}

pub fn get_repo_path(mut path: String) -> String {
    if path.ends_with(".git") {
        path = path[..path.len() - 4].to_string();
    }
    path.to_string()
}

pub async fn get_git_model_by_path(context: Context, path: String) -> Option<git_repo::Model> {
    let git_model = context
        .services
        .git_db_storage
        .find_git_repo_exact_match(get_repo_path(path).as_str())
        .await;

    git_model.unwrap_or_default()
}

const LFS_VERSION: &str = "https://git-lfs.github.com/spec/v1";
/// This is the original & default transfer adapter. All Git LFS clients and servers SHOULD support it.
pub const LFS_TRANSFER_API: &str = "basic";
pub const LFS_HASH_ALGO: &str = "sha256";
const LFS_OID_LEN: usize = 64;
const LFS_POINTER_MAX_SIZE: usize = 300; // bytes

pub fn parse_pointer_data(data: &[u8]) -> Option<(String, u64)> {
    if data.len() > LFS_POINTER_MAX_SIZE {
        return None;
    }
    // Start with format `version ...`
    if let Some(data) =
        data.strip_prefix(format!("version {}\noid {}:", LFS_VERSION, LFS_HASH_ALGO).as_bytes())
    {
        if data.len() > LFS_OID_LEN && data[LFS_OID_LEN] == b'\n' {
            // check `oid` length
            let oid = String::from_utf8(data[..LFS_OID_LEN].to_vec()).unwrap();
            if let Some(data) = data.strip_prefix(format!("{}\nsize ", oid).as_bytes()) {
                let data = String::from_utf8(data[..].to_vec()).unwrap();
                if let Ok(size) = data.trim_end().parse::<u64>() {
                    return Some((oid, size));
                }
            }
        }
    }
    None
}
