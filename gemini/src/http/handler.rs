use common::model::CommonResult;
use jupiter::context::Context;

use crate::{
    util::{get_git_model_by_path, repo_alias_to_identifier},
    ztm::{
        agent::share_repo, get_or_create_remote_mega_tunnel, send_get_request_to_peer_by_tunnel,
    },
    RepoInfo,
};

pub async fn repo_provide(
    bootstrap_node: String,
    context: Context,
    path: String,
    alias: String,
    origin: String,
) -> Result<String, String> {
    let url = format!("{bootstrap_node}/api/v1/repo_provide");

    let git_model = match get_git_model_by_path(context.clone(), path).await {
        Some(r) => r,
        None => return Err(String::from("Repo not found")),
    };

    let git_ref = context
        .services
        .git_db_storage
        .get_default_ref(git_model.id)
        .await
        .unwrap()
        .unwrap();

    let name = git_model.repo_name;
    let identifier = repo_alias_to_identifier(alias).await;
    let update_time = git_model.created_at.and_utc().timestamp();
    let repo_info = RepoInfo {
        name,
        identifier,
        origin,
        update_time,
        commit: git_ref.ref_git_id,
        peer_online: true,
    };
    share_repo(url.clone(), repo_info).await;
    Ok("success".to_string())
}

pub async fn repo_folk_alias(ztm_agent_port: u16, identifier: String) -> Result<String, String> {
    let remote_peer_id = match get_peer_id_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    let alias = match get_alias_from_identifier(identifier) {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    let path = match get_git_path_by_alias(alias, remote_peer_id.clone(), ztm_agent_port).await {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

    let local_port = get_or_create_remote_mega_tunnel(ztm_agent_port, remote_peer_id).await;

    let local_port = match local_port {
        Ok(local_port) => local_port,
        Err(e) => {
            return Err(e);
        }
    };

    let msg = format!("http://localhost:{local_port}{path}.git");
    Ok(msg)
}

pub fn get_peer_id_from_identifier(identifier: String) -> Result<String, String> {
    // p2p://mrJ46F8gd2sa2Dx3iCYf6DauJ2WpAaepus7PwyZVebgD/8000/third-part/mega_143.git
    let words: Vec<&str> = identifier.split('/').collect();
    if words.len() <= 2 {
        return Err("invalid identifier".to_string());
    }
    Ok(words.get(2).unwrap().to_string())
}

pub fn get_alias_from_identifier(identifier: String) -> Result<String, String> {
    // p2p://wGg2inNE22LY1eHttDB63znw2MnsK8CPXeG2nfhpXs5a/serde_python
    let words: Vec<&str> = identifier.split('/').collect();
    if words.len() <= 3 {
        return Err("invalid identifier".to_string());
    }
    Ok(words.get(3).unwrap().to_string())
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
