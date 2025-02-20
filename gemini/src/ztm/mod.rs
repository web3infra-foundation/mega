use agent::{LocalZTMAgent, ZTMAgent};
use reqwest::{header::CONTENT_TYPE, Client};

use crate::util::{get_available_port, get_ztm_app_tunnel_bound_name, handle_response};

pub mod agent;
pub mod hub;

const MESH_NAME: &str = "relay_mesh";

const ZTM_APP_PROVIDER: &str = "mega";

const ZTM_APP_NAME: &str = "tunnel_punch";

async fn create_tunnel(
    ztm_agent_port: u16,
    remote_peer_id: String,
    local_port: u16,
    remote_port: u16,
    bound_name: String,
) -> Result<(), String> {
    let agent: LocalZTMAgent = LocalZTMAgent {
        agent_port: ztm_agent_port,
    };
    let local_ep = match agent.get_ztm_local_endpoint().await {
        Ok(ep) => ep,
        Err(e) => return Err(e),
    };
    let remote_ep = match agent.get_ztm_remote_endpoint(remote_peer_id.clone()).await {
        Ok(ep) => ep,
        Err(e) => return Err(e),
    };

    tracing::info!("create_tunnel remote_ep:{:?}", remote_ep);

    //creata inbound
    match agent
        .create_ztm_app_tunnel_inbound(
            local_ep.id,
            ZTM_APP_PROVIDER.to_string(),
            ZTM_APP_NAME.to_string(),
            bound_name.clone(),
            local_port,
        )
        .await
    {
        Ok(_) => (),
        Err(s) => {
            tracing::error!("create app inbound failed, {s}");
            return Err(s);
        }
    }
    tracing::info!("create app inbound successfully");

    //creata outbound
    match agent
        .create_ztm_app_tunnel_outbound(
            remote_ep.id,
            ZTM_APP_PROVIDER.to_string(),
            ZTM_APP_NAME.to_string(),
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
            return Err(s);
        }
    }
    Ok(())
}

pub async fn get_or_create_remote_mega_tunnel(
    ztm_agent_port: u16,
    remote_peer_id: String,
) -> Result<u16, String> {
    let bound_name = get_ztm_app_tunnel_bound_name(remote_peer_id.clone()).await;

    //Check if the tunnel exists
    let local_port = search_tunnel_inbound_port(ztm_agent_port, bound_name.clone()).await;

    tracing::debug!(
        "get_or_create_remote_mega_tunnel, local_port exist:{:?}",
        local_port
    );

    let local_port = match local_port {
        Some(local_port) => local_port,
        None => {
            //create a new tunnel to remote peer
            let local_port = match get_available_port() {
                Ok(port) => port,
                Err(e) => {
                    return Err(e);
                }
            };
            let remote_port = 8000;
            match create_tunnel(
                ztm_agent_port,
                remote_peer_id.clone(),
                local_port,
                remote_port,
                bound_name.clone(),
            )
            .await
            {
                Ok(_) => {
                    tracing::info!(
                        "create ztm tunnel successfully: \nbound_name:{}\nlocal port:{}\nremote peer:{}\nremote port:{}",
                        bound_name,
                        local_port,
                        remote_peer_id,
                        remote_port,
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "create ztm tunnel failed: \nbound_name:{}\nlocal port:{}\nremote peer:{}\nremote port:{}\nerror:{}",
                        bound_name,
                        local_port,
                        remote_peer_id,
                        remote_port,
                        e.clone(),
                    );
                    return Err(e);
                }
            }
            local_port
        }
    };
    Ok(local_port)
}

async fn search_tunnel_inbound_port(ztm_agent_port: u16, bound_name: String) -> Option<u16> {
    let agent: LocalZTMAgent = LocalZTMAgent {
        agent_port: ztm_agent_port,
    };
    let local_ep = match agent.get_ztm_local_endpoint().await {
        Ok(ep) => ep,
        Err(_) => return None,
    };
    tracing::debug!("local_ep:{:?}", local_ep);
    //search inbound
    agent
        .get_ztm_app_tunnel_inbound_port(
            local_ep.id,
            ZTM_APP_PROVIDER.to_string(),
            ZTM_APP_NAME.to_string(),
            bound_name.clone(),
        )
        .await
}

pub async fn send_get_request_to_peer_by_tunnel(
    ztm_agent_port: u16,
    remote_peer_id: String,
    path: String,
) -> Result<String, String> {
    let local_port = get_or_create_remote_mega_tunnel(ztm_agent_port, remote_peer_id).await;

    let local_port = match local_port {
        Ok(local_port) => local_port,
        Err(e) => {
            return Err(e);
        }
    };

    let url = format!("http://127.0.0.1:{local_port}/{path}");
    let request_result = reqwest::get(url.clone()).await;
    match handle_response(request_result).await {
        Ok(s) => {
            tracing::info!("get response from url {}:\n{}", url, s.clone());
            Ok(s)
        }
        Err(e) => {
            tracing::error!("get response from url {} failed:\n{}", url, e);
            Err(e)
        }
    }
}

pub async fn send_post_request_to_peer_by_tunnel(
    ztm_agent_port: u16,
    remote_peer_id: String,
    path: String,
    body: String,
) -> Result<String, String> {
    let local_port = get_or_create_remote_mega_tunnel(ztm_agent_port, remote_peer_id).await;

    let local_port = match local_port {
        Ok(local_port) => local_port,
        Err(e) => {
            return Err(e);
        }
    };

    let url = format!("http://127.0.0.1:{local_port}/{path}");

    let client = Client::new();

    let request_result = client
        .post(url.clone())
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await;
    match handle_response(request_result).await {
        Ok(s) => {
            tracing::info!("post response from url {}:\n{}", url, s.clone());
            Ok(s)
        }
        Err(e) => {
            tracing::error!("post response from url {} failed:\n{}", url, e);
            Err(e)
        }
    }
}
