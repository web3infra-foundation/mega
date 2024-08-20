use common::model::CommonResult;
use jupiter::context::Context;

use crate::{
    util::{get_available_port, get_short_peer_id, repo_alias_to_identifier},
    ztm::{agent::share_repo, create_tunnel, send_get_request_to_peer_by_tunnel},
    RepoInfo,
};

pub async fn repo_provide(
    bootstrap_node: String,
    context: Context,
    path: String,
    alias: String,
) -> Result<String, String> {
    let url = format!("{bootstrap_node}/api/v1/repo_provide");
    let git_model = context
        .services
        .git_db_storage
        .find_git_repo_exact_match(path.as_str())
        .await;

    let git_model = match git_model {
        Ok(r) => {
            if let Some(m) = r {
                m
            } else {
                return Err(String::from("Repo not found"));
            }
        }
        Err(_) => return Err(String::from("Repo not found")),
    };
    let git_ref = context
        .services
        .git_db_storage
        .get_default_ref(git_model.id)
        .await
        .unwrap()
        .unwrap();

    let name = git_model.repo_name;
    let (peer_id, _) = vault::init();
    let identifier = repo_alias_to_identifier(alias);
    let update_time = git_model.created_at.and_utc().timestamp();
    let repo_info = RepoInfo {
        name,
        identifier,
        origin: peer_id,
        update_time,
        commit: git_ref.ref_git_id,
        peer_online: true,
    };
    share_repo(url.clone(), repo_info).await;
    Ok("success".to_string())
}

pub async fn repo_folk(
    ztm_agent_port: u16,
    identifier: String,
    local_port: u16,
) -> Result<String, String> {
    let remote_peer_id = match get_peer_id_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    let remote_port = match get_remote_port_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    let git_path = match get_git_path_from_identifier(identifier) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    let (peer_id, _) = vault::init();
    let bound_name = format!(
        "{}_{}",
        get_short_peer_id(peer_id),
        get_short_peer_id(remote_peer_id.clone())
    );
    match create_tunnel(
        ztm_agent_port,
        remote_peer_id,
        local_port,
        remote_port,
        bound_name,
    )
    .await
    {
        Ok(_) => (),
        Err(e) => return Err(e),
    }

    let msg = format!("git clone http://localhost:{local_port}/{git_path}");
    Ok(msg)
}

pub async fn repo_folk_alias(ztm_agent_port: u16, identifier: String) -> Result<String, String> {
    let remote_peer_id = match get_peer_id_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    let remote_port = 8000;
    let alias = match get_alias_from_identifier(identifier) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    let path = match get_git_path_by_alias(alias, remote_peer_id.clone(), ztm_agent_port).await {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

    let peer_id = vault::get_peerid();
    let bound_name = format!(
        "{}_{}",
        get_short_peer_id(peer_id),
        get_short_peer_id(remote_peer_id.clone())
    );
    let local_port = match get_available_port() {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    match create_tunnel(
        ztm_agent_port,
        remote_peer_id,
        local_port,
        remote_port,
        bound_name,
    )
    .await
    {
        Ok(_) => (),
        Err(e) => return Err(e),
    }

    let msg = format!("git clone http://localhost:{local_port}{path}.git");
    Ok(msg)
}

pub fn get_peer_id_from_identifier(identifier: String) -> Result<String, String> {
    // p2p://mrJ46F8gd2sa2Dx3iCYf6DauJ2WpAaepus7PwyZVebgD/8000/third-part/mega_143.git
    let words: Vec<&str> = identifier.split('/').collect();
    if words.len() <= 2 {
        return Err("invalid identifier".to_string());
    }
    return Ok(words.get(2).unwrap().to_string());
}

pub fn get_remote_port_from_identifier(identifier: String) -> Result<u16, String> {
    // p2p://mrJ46F8gd2sa2Dx3iCYf6DauJ2WpAaepus7PwyZVebgD/8000/third-part/mega_143.git
    let words: Vec<&str> = identifier.split('/').collect();
    if words.len() <= 3 {
        return Err("invalid identifier".to_string());
    }
    match words.get(3).unwrap().parse::<u16>() {
        Ok(number) => Ok(number),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_git_path_from_identifier(identifier: String) -> Result<String, String> {
    // p2p://mrJ46F8gd2sa2Dx3iCYf6DauJ2WpAaepus7PwyZVebgD/8000/third-part/mega_143.git
    let words: Vec<&str> = identifier.split('/').collect();
    if words.len() <= 4 {
        return Err("invalid identifier".to_string());
    }
    let path = words[4..].join("/");
    Ok(path)
}

pub fn get_alias_from_identifier(identifier: String) -> Result<String, String> {
    // p2p://wGg2inNE22LY1eHttDB63znw2MnsK8CPXeG2nfhpXs5a/serde_python
    let words: Vec<&str> = identifier.split('/').collect();
    if words.len() <= 3 {
        return Err("invalid identifier".to_string());
    }
    return Ok(words.get(3).unwrap().to_string());
}

pub async fn get_git_path_by_alias(
    alias: String,
    peer_id: String,
    ztm_agent_port: u16,
) -> Result<String, String> {
    let path = format!("api/v1/mega/ztm/alias_to_path?alias={}", alias);
    let result = match send_get_request_to_peer_by_tunnel(ztm_agent_port, peer_id, path).await {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    let common_result: CommonResult<String> = match serde_json::from_str(result.as_str()) {
        Ok(s) => s,
        Err(e) => return Err(e.to_string()),
    };
    match common_result.data {
        Some(path) => Ok(path),
        None => Err("Path not found".to_string()),
    }
}

#[cfg(test)]
mod tests {}
