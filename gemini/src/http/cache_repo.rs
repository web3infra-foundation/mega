use std::{fs, path::PathBuf, time::Duration};

use callisto::ztm_path_mapping;
use common::utils::generate_id;
use jupiter::context::Context;
use tokio::process::Command;
use vault::get_peerid;

use crate::{
    util::{get_git_model_by_path, handle_response},
    ztm::agent::LocalZTMAgent,
    RepoInfo,
};

use super::handler::{repo_folk_alias, repo_provide};

pub async fn cache_public_repository(
    bootstrap_node: String,
    context: Context,
    agent: LocalZTMAgent,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));
    loop {
        interval.tick().await;
        cache_public_repository_handler(bootstrap_node.clone(), context.clone(), agent.clone())
            .await;
    }
}

async fn cache_public_repository_handler(
    bootstrap_node: String,
    context: Context,
    agent: LocalZTMAgent,
) {
    tracing::info!("Start caching public repositories");
    // get public repo by bootstrap_node
    let url = format!("{bootstrap_node}/api/v1/repo_list");
    let request_result = reqwest::get(url.clone()).await;
    let response_text = match handle_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            tracing::error!("GET {url} failed,{s}");
            return;
        }
    };
    let repo_list: Vec<RepoInfo> = match serde_json::from_slice(response_text.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("{}", e);
            return;
        }
    };
    for repo in repo_list {
        if !repo.peer_online && repo.origin != get_peerid() {
            continue;
        }
        let bootstrap_node = bootstrap_node.clone();
        let context = context.clone();
        let agent = agent.clone();
        let handle = tokio::spawn(async {
            clone_and_share_repo(bootstrap_node, context, agent, repo).await;
        });
        handle.await.unwrap();
    }
}

async fn clone_and_share_repo(
    bootstrap_node: String,
    context: Context,
    agent: LocalZTMAgent,
    repo: RepoInfo,
) {
    let alias = repo.name;
    let repo_path = format!("/third-part/{}", alias.clone());

    if get_git_model_by_path(context.clone(), repo_path)
        .await
        .is_some()
    {
        //exist
        //TODO update if origin commit change
        tracing::info!("Repository {} exists", alias);
        return;
    }

    let res: Option<ztm_path_mapping::Model> = context
        .services
        .ztm_storage
        .get_path_from_alias(&alias)
        .await
        .unwrap();
    if res.is_none() {
        //clone repo from other peer
        let clone_url = repo_folk_alias(agent.agent_port, repo.identifier.to_string()).await;
        let clone_url = match clone_url {
            Ok(d) => d,
            Err(_) => return,
        };
        tracing::info!("Clone {} with local port: {}", repo.identifier, clone_url);
        match clone_repository(clone_url, alias.clone()).await {
            Ok(_) => {
                tracing::info!("Clone {} to local successfully", repo.identifier);
                let _ = share_repository(alias, context, bootstrap_node, repo.origin).await;
            }
            Err(e) => {
                tracing::error!("Clone {} to local failed:{}", repo.identifier, e);
            }
        }
    }
}

async fn clone_repository(repo_url: String, name: String) -> Result<(), String> {
    let base_dir =
        PathBuf::from(std::env::var("MEGA_BASE_DIR").unwrap_or_else(|_| "/tmp/.mega".to_string()));
    let target_dir = base_dir.join("tmp").join(name.clone());

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).unwrap();
    }

    tracing::info!(
        "Exec: git clone {} {}",
        repo_url,
        target_dir.to_str().unwrap()
    );

    let mut cmd = Command::new("git")
        .arg("clone")
        .arg(repo_url)
        .arg(target_dir.to_str().unwrap())
        .spawn()
        .map_err(|e| format!("Failed to execute process: {}", e))?;

    let status = cmd
        .wait()
        .await
        .map_err(|e| format!("Failed to execute process: {}", e))?;

    tracing::info!("Git clone with result: {}", status);

    change_remote_url(target_dir.clone(), name).await?;
    push_to_new_remote(target_dir).await?;
    Ok(())
}

async fn change_remote_url(repo_path: PathBuf, name: String) -> Result<(), String> {
    tracing::info!("Exec: git remote remove origin");
    let _output = Command::new("git")
        .arg("remote")
        .arg("remove")
        .arg("origin")
        .current_dir(repo_path.clone())
        .output()
        .await
        .map_err(|e| format!("Failed to execute process: {}", e))?;

    let new_path = &format!("http://localhost:8000/third-part/{}", name);
    tracing::info!("Exec: git remote add origin {}", new_path);
    let _output = Command::new("git")
        .arg("remote")
        .arg("add")
        .arg("origin")
        .arg(new_path)
        .current_dir(repo_path.clone())
        .output()
        .await
        .map_err(|e| format!("Failed to execute process: {}", e))?;

    Ok(())
}

async fn push_to_new_remote(repo_path: PathBuf) -> Result<(), String> {
    tracing::info!("Exec: git push origin master");
    let mut cmd = Command::new("git")
        .arg("push")
        .arg("origin")
        .arg("master")
        .current_dir(repo_path.clone())
        .spawn()
        .map_err(|e| format!("Failed to execute process: {}", e))?;

    let status = cmd
        .wait()
        .await
        .map_err(|e| format!("Failed to execute process: {}", e))?;

    tracing::info!("Git push with result: {}", status);

    Ok(())
}

async fn share_repository(
    name: String,
    context: Context,
    bootstrap_node: String,
    origin: String,
) -> Result<(), String> {
    let repo_path = format!("/third-part/{}", name);
    let model: ztm_path_mapping::Model = ztm_path_mapping::Model {
        id: generate_id(),
        alias: name.clone(),
        repo_path: repo_path.to_string(),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };
    context
        .services
        .ztm_storage
        .save_alias_mapping(model.clone())
        .await
        .map_err(|e| format!("{}", e))?;
    let res = repo_provide(bootstrap_node, context, repo_path.clone(), name, origin).await;
    match res {
        Ok(_) => {
            tracing::info!("Share repo {} successfully", repo_path);
        }
        Err(e) => {
            tracing::error!(e);
        }
    }
    Ok(())
}
