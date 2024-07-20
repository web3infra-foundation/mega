use axum::{body::Body, http::Response};
use common::model::GetParams;
use jupiter::context::Context;
use reqwest::StatusCode;
use venus::import_repo::repo::Repo;

use crate::{
    ztm::agent::{share_repo, LocalZTMAgent, ZTMAgent},
    RepoInfo, ZTM_APP_PROVIDER,
};

pub async fn repo_provide(
    port: u16,
    bootstrap_node: Option<String>,
    context: Context,
    params: GetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    let bootstrap_node_clone = match bootstrap_node {
        Some(b) => b.clone(),
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Bootstrap node not provide\n"),
            ));
        }
    };
    let path = match params.path.clone() {
        Some(p) => p,
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("Path not provide\n")));
        }
    };
    let url = format!("{bootstrap_node_clone}/api/v1/repo_provide");
    let git_model = context
        .services
        .git_db_storage
        .find_git_repo_by_path(path.as_str())
        .await;

    let git_model = match git_model {
        Ok(r) => {
            if let Some(m) = r {
                m
            } else {
                return Err((StatusCode::BAD_REQUEST, String::from("Repo not found")));
            }
        }
        Err(_) => return Err((StatusCode::BAD_REQUEST, String::from("Repo not found"))),
    };
    let repo: Repo = git_model.clone().into();
    let git_ref = context
        .services
        .git_db_storage
        .get_default_ref(&repo)
        .await
        .unwrap()
        .unwrap();

    let name = git_model.repo_name;
    let repo_path = git_model.repo_path;
    let (peer_id, _) = vault::init();
    let identifier = format!("p2p://{}/{port}{repo_path}.git", peer_id.clone());
    let update_time = git_model.created_at.and_utc().timestamp();
    let repo_info = RepoInfo {
        name,
        identifier,
        origin: peer_id.clone(),
        update_time,
        commit: git_ref.ref_hash,
        peer_online: true,
    };
    share_repo(url.clone(), repo_info).await;
    Ok(Response::builder().body(Body::from("success")).unwrap())
}

pub async fn repo_folk(
    ztm_agent_port: u16,
    params: GetParams,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("params:{:?}", params);
    let identifier = match params.identifier.clone() {
        Some(i) => i,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                String::from("Identifier not provide\n"),
            ));
        }
    };
    let port = match params.port {
        Some(i) => i,
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("Port not provide\n")));
        }
    };
    let remote_peer_id = match get_peer_id_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(e) => return Err((StatusCode::BAD_REQUEST, e)),
    };
    let remote_port = match get_remote_port_from_identifier(identifier.clone()) {
        Ok(p) => p,
        Err(e) => return Err((StatusCode::BAD_REQUEST, e)),
    };
    let git_path = match get_git_path_from_identifier(identifier) {
        Ok(p) => p,
        Err(e) => return Err((StatusCode::BAD_REQUEST, e)),
    };

    let agent: LocalZTMAgent = LocalZTMAgent {
        agent_port: ztm_agent_port,
    };
    let local_ep = match agent.get_ztm_local_endpoint().await {
        Ok(ep) => ep,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    };

    let remote_ep = match agent.get_ztm_remote_endpoint(remote_peer_id.clone()).await {
        Ok(ep) => ep,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    };

    let (peer_id, _) = vault::init();
    let bound_name = format!(
        "{}_{}",
        get_short_peer_id(peer_id),
        get_short_peer_id(remote_peer_id)
    );
    //creata inbound
    match agent
        .create_ztm_app_tunnel_inbound(
            local_ep.id,
            ZTM_APP_PROVIDER.to_string(),
            "tunnel".to_string(),
            bound_name.clone(),
            port,
        )
        .await
    {
        Ok(_) => (),
        Err(s) => {
            tracing::error!("create app inbound, {s}");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, s));
        }
    }
    tracing::info!("create app inbound successfully");

    //creata outbound
    match agent
        .create_ztm_app_tunnel_outbound(
            remote_ep.id,
            ZTM_APP_PROVIDER.to_string(),
            "tunnel".to_string(),
            bound_name,
            remote_port,
        )
        .await
    {
        Ok(msg) => {
            tracing::info!("create app outbound successfully,{}", msg);
        }
        Err(s) => {
            tracing::error!("create app outbound, {s}");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, s));
        }
    }
    let msg = format!("Success, you can try to clone the repo like this:\ngit clone http://localhost:{port}/{git_path}");
    Ok(Response::builder().body(Body::from(msg)).unwrap())
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

pub fn get_short_peer_id(peer_id: String) -> String {
    if peer_id.len() <= 7 {
        return peer_id;
    }
    peer_id[0..7].to_string()
}

#[cfg(test)]
mod tests {}
