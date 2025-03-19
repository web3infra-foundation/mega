use std::{
    fs::{self, create_dir_all, File},
    io::Read,
    path::PathBuf,
    time::Duration,
};

use crate::{
    http::handler::{repo_folk_alias, repo_provide},
    lfs::share_lfs,
    util::{get_git_model_by_path, handle_response},
    ztm::{agent::LocalZTMAgent, get_or_create_remote_mega_tunnel},
    LFSInfo, RepoInfo,
};
use callisto::ztm_path_mapping;
use ceres::lfs::lfs_structs::FetchchunkResponse;
use common::utils::generate_id;
use jupiter::context::Context;
use reqwest::{get, Client};
use ring::digest::SHA256;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, process::Command};
use vault::get_peerid;

pub async fn cache_public_repo_and_lfs(
    bootstrap_node: String,
    context: Context,
    agent: LocalZTMAgent,
    http_port: u16,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));
    loop {
        interval.tick().await;
        cache_public_repository_handler(
            bootstrap_node.clone(),
            context.clone(),
            agent.clone(),
            http_port,
        )
        .await;
        cache_public_lfs_handler(
            bootstrap_node.clone(),
            context.clone(),
            agent.clone(),
            http_port,
        )
        .await;
    }
}

async fn cache_public_repository_handler(
    bootstrap_node: String,
    context: Context,
    agent: LocalZTMAgent,
    http_port: u16,
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
        if !repo.peer_online && repo.origin != get_peerid().await {
            continue;
        }
        let bootstrap_node = bootstrap_node.clone();
        let context = context.clone();
        let agent = agent.clone();
        let handle = tokio::spawn(async move {
            clone_and_share_repo(bootstrap_node, context, agent, repo, http_port).await;
        });
        handle.await.unwrap();
    }
}

async fn clone_and_share_repo(
    bootstrap_node: String,
    context: Context,
    agent: LocalZTMAgent,
    repo: RepoInfo,
    http_port: u16,
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
        match clone_repository(clone_url, alias.clone(), http_port).await {
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

async fn clone_repository(repo_url: String, name: String, http_port: u16) -> Result<(), String> {
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

    change_remote_url(target_dir.clone(), name, http_port).await?;
    push_to_new_remote(target_dir).await?;
    Ok(())
}

async fn change_remote_url(repo_path: PathBuf, name: String, http_port: u16) -> Result<(), String> {
    tracing::info!("Exec: git remote remove origin");
    let _output = Command::new("git")
        .arg("remote")
        .arg("remove")
        .arg("origin")
        .current_dir(repo_path.clone())
        .output()
        .await
        .map_err(|e| format!("Failed to execute process: {}", e))?;

    let new_path = &format!("http://localhost:{}/third-part/{}", http_port, name);
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

async fn cache_public_lfs_handler(
    bootstrap_node: String,
    context: Context,
    agent: LocalZTMAgent,
    http_port: u16,
) {
    tracing::info!("Start caching public lfs");
    // get public lfs by bootstrap_node
    let url = format!("{bootstrap_node}/api/v1/lfs_list");
    let request_result = reqwest::get(url.clone()).await;
    let response_text = match handle_response(request_result).await {
        Ok(s) => s,
        Err(s) => {
            tracing::error!("GET {url} failed,{s}");
            return;
        }
    };
    let lfs_list: Vec<LFSInfo> = match serde_json::from_slice(response_text.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("{}", e);
            return;
        }
    };
    for lfs in lfs_list {
        if !lfs.peer_online || lfs.peer_id == get_peerid().await {
            continue;
        }
        let lfs_object = context
            .lfs_stg()
            .get_lfs_object(&lfs.file_hash)
            .await
            .unwrap();
        if lfs_object.is_some() {
            tracing::info!("lfs {} exists, skip", lfs.file_hash);
            continue;
        }
        let bootstrap_node = bootstrap_node.clone();
        let agent = agent.clone();
        let handle = tokio::spawn(async move {
            download_and_upload_lfs(bootstrap_node, agent, lfs, http_port).await;
        });
        handle.await.unwrap();
    }
}

async fn download_and_upload_lfs(
    bootstrap_node: String,
    agent: LocalZTMAgent,
    lfs: LFSInfo,
    http_port: u16,
) {
    let file_hash = lfs.file_hash.clone();
    let local_port =
        match get_or_create_remote_mega_tunnel(agent.agent_port, lfs.peer_id.clone()).await {
            Ok(local_port) => local_port,
            Err(e) => {
                tracing::error!("Open tunnel to {} failed,{}", lfs.peer_id, e);
                return;
            }
        };

    download_lfs_by_chunk(local_port, lfs.clone()).await;

    if !checksum_lfs(lfs.clone()) {
        return;
    }

    //upload to local mega lfs
    let client = Client::new();
    let json_str = format!(
        r#"
        {{
            "operation": "upload",
            "transfers": [],
            "hash_algo": "",
            "objects": [
                {{
                    "oid":"{}",
                    "size":{}
                }}
            ]
        }}"#,
        file_hash, lfs.file_size
    );

    let url = format!("http://localhost:{}/objects/batch", http_port);
    let response = client
        .post(url)
        .header("content-type", "application/json")
        .body(json_str)
        .send()
        .await
        .unwrap();

    if !response.status().is_success() {
        tracing::error!("Failed to send POST request: HTTP {}", response.status());
        return;
    }

    let target_path = match get_lfs_tmp_file(lfs.clone()) {
        Some(p) => p,
        None => {
            return;
        }
    };

    let mut file = File::open(target_path.clone()).unwrap();
    let mut file_content = Vec::new();
    file.read_to_end(&mut file_content).unwrap();

    let url = format!(
        "http://localhost:{}/objects/{}",
        http_port,
        file_hash.clone()
    );
    let response = client.put(url).body(file_content).send().await.unwrap();

    if response.status().is_success() {
        tracing::info!("LFS {} uploaded successfully!", file_hash);
    } else {
        tracing::error!("Failed to upload file: HTTP {}", response.status());
    }

    fs::remove_file(target_path).unwrap();

    share_lfs(
        bootstrap_node,
        lfs.file_hash,
        lfs.hash_type,
        lfs.file_size,
        lfs.origin,
    )
    .await;
}

async fn download_lfs_by_chunk(local_port: u16, lfs: LFSInfo) {
    tracing::info!("Prepare to download LFS {} by chunks", lfs.file_hash);
    // fetch chunks info http://localhost:{localport}/objects/{object_id}/chunks
    let url = format!(
        "http://localhost:{}/objects/{}/chunks",
        local_port, lfs.file_hash
    );
    let chunk_info = match get(url.clone()).await {
        Ok(response) => {
            if !response.status().is_success() {
                tracing::error!("Get lfs chuncks info failed  {}", url);
                return;
            }
            let body = response.text().await.unwrap();
            let chunck_info: FetchchunkResponse = serde_json::from_str(&body).unwrap();
            chunck_info
        }
        Err(_) => {
            tracing::error!("Get lfs chuncks info failed {}", url);
            return;
        }
    };
    let mut chunks = chunk_info.chunks;
    chunks.sort_unstable_by(|a, b| a.offset.cmp(&b.offset));

    // create temp file
    let base_dir =
        PathBuf::from(std::env::var("MEGA_BASE_DIR").unwrap_or_else(|_| "/tmp/.mega".to_string()));
    let base_dir = base_dir.join("tmp");
    if !base_dir.exists() {
        create_dir_all(base_dir.clone()).unwrap();
    }
    let target_path = base_dir.join(lfs.file_hash.clone());
    if target_path.exists() {
        fs::remove_file(&target_path).unwrap();
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(target_path)
        .await
        .unwrap();

    tracing::info!(
        "Start to download LFS chunks,size: {}B, chunk_num: {}",
        chunk_info.size,
        chunks.len()
    );
    for (index, chunk) in chunks.iter().enumerate() {
        // http://localhost:{localport}/objects/{object_id}/chunks
        let url = format!("http://localhost:{}/objects/{}", local_port, chunk.sub_oid);
        let data = match get(url.clone()).await {
            Ok(response) => {
                if !response.status().is_success() {
                    tracing::error!("Get lfs chuncks info failed  {}", url);
                    return;
                }
                response.bytes().await.unwrap()
            }
            Err(_) => {
                tracing::error!("Get lfs chuncks info failed {}", url);
                return;
            }
        };
        file.write_all(&data).await.unwrap();
        tracing::info!("Chunk[{}] download from {} successfully", index, url);
    }
    tracing::info!("Download LFS {} by chunks successfully", lfs.file_hash);
}

fn checksum_lfs(lfs: LFSInfo) -> bool {
    tracing::info!("Prepare to check LFS SHA256");
    let target_path = match get_lfs_tmp_file(lfs.clone()) {
        Some(p) => p,
        None => {
            return false;
        }
    };
    let mut file = File::open(target_path).unwrap();

    let mut context = ring::digest::Context::new(&SHA256);
    let mut buffer = [0u8; 1024];

    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    let checksum = hex::encode(context.finish().as_ref());
    let result = checksum == lfs.file_hash;
    if result {
        tracing::info!("Check LFS SHA256({}) successfully", lfs.file_hash);
    } else {
        tracing::error!("Check LFS SHA256 failed");
    }
    result
}

fn get_lfs_tmp_file(lfs: LFSInfo) -> Option<PathBuf> {
    let base_dir =
        PathBuf::from(std::env::var("MEGA_BASE_DIR").unwrap_or_else(|_| "/tmp/.mega".to_string()));
    let base_dir = base_dir.join("tmp");
    if !base_dir.exists() {
        create_dir_all(base_dir.clone()).unwrap();
    }
    let target_path = base_dir.join(lfs.file_hash.clone());
    if !target_path.exists() {
        return None;
    }
    Some(target_path)
}

#[cfg(test)]
mod tests {
    // use std::{fs::File, io::Read};

    // use reqwest::Client;

    // #[tokio::test]
    // async fn lfs_upload() {
    //     let client = Client::new();
    //     let json_str = r#"
    //     {
    //         "operation": "upload",
    //         "transfers": [],
    //         "hash_algo": "",
    //         "objects": [
    //             {
    //                 "oid":"52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6",
    //                 "size":199246498
    //             }
    //         ]
    //     }"#;

    //     let response = client
    //         .post("http://222.20.126.106:8000/objects/batch")
    //         .header("content-type", "application/json")
    //         .body(json_str)
    //         .send()
    //         .await
    //         .unwrap();

    //     if response.status().is_success() {
    //         println!("POST request successful!");
    //     } else {
    //         println!("Failed to send POST request: HTTP {}", response.status());
    //         return;
    //     }

    //     let url = "http://222.20.126.106:8000/objects/52c90a86cb034b7a1c4beb79304fa76bd0a6cbb7b168c3a935076c714bd1c6b6";
    //     let client = Client::new();

    //     let mut file = File::open("/home/wujian/pr/mega/bbb.mp4").unwrap();
    //     let mut file_content = Vec::new();
    //     file.read_to_end(&mut file_content).unwrap();

    //     let response = client.put(url).body(file_content).send().await.unwrap();

    //     if response.status().is_success() {
    //         println!("File uploaded successfully!");
    //     } else {
    //         println!("Failed to upload file: HTTP {}", response.status());
    //     }
    // }
}
