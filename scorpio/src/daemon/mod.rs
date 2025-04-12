use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::Router;
use axum::routing::{post, get};
use mercury::hash::SHA1;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use crate::fuse::MegaFuse;
use crate::manager::fetch::fetch;
use crate::manager::{ScorpioManager, WorkDir};
use crate::util::{scorpio_config, GPath};
mod git;
const SUCCESS: &str   = "Success";
const FAIL : &str   = "Fail";

#[derive(Debug, Deserialize, Serialize)]
struct MountRequest{
    path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MountResponse {
    status: String,
    mount: MountInfo,
    message: String,
}
#[derive(Debug, Deserialize, Serialize,Default)]
struct MountInfo {
    hash: String,
    path: String,
    inode: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct MountsResponse {
    status: String,
    mounts: Vec<MountInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UmountRequest {
    path: Option<String>,
    inode: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UmountResponse {
    status: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfigResponse {
    status: String,
    config: ConfigInfo,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfigInfo {
    mega_url: String,
    mount_path: String,
    store_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfigRequest {
    mega_url: Option<String>,
    mount_path: Option<String>,
    store_path: Option<String>,
}
#[derive(Clone)]
struct ScoState{
    fuse:Arc<MegaFuse>,
    manager:Arc<Mutex<ScorpioManager>>,
}
#[allow(unused)]
pub async fn daemon_main(fuse:Arc<MegaFuse>,manager:ScorpioManager) {
    let inner = ScoState{
        fuse,
        manager: Arc::new(Mutex::new(manager)),
    };
    let mut app = Router::new()
        .route("/api/fs/mount", post(mount_handler))
        .route("/api/fs/mpoint", get(mounts_handler))
        .route("/api/fs/umount", post(umount_handler))
        .route("/api/config", get(config_handler))
        .route("/api/config", post(update_config_handler))
        .route("/api/git/status", get(git::git_status_handler))
        .route("/api/git/commit", post(git::git_commit_handler))
        .route("/api/git/push", post(git::git_push_handler))
        .route("/api/git/add", post(git::git_add_handler))
        .route("/api/git/reset", post(git::git_reset_handler))
        .with_state(inner);

    // LFS route & merge it
    let lfs_route = crate::scolfs::route::router();
    let app = app.merge(lfs_route);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:2725").await.unwrap();
    axum::serve(listener, app).await.unwrap()
}

/// Mount a dictionary by path , like "/path/to/dic" or "path/to/dic"
async fn mount_handler(
    State(state): State<ScoState>, 
    req: axum::Json<MountRequest>,
) -> axum::Json<MountResponse> {

    // transform by GPath , is case of wrong format.
    let mono_path = GPath::from(req.path.clone()).to_string();

    // bool to indicate if it is a temp path for buck2.
    let mut temp_mount = false;
    // get inode by this path .
    let inode = match state.fuse.get_inode(&mono_path).await{
        Ok(a) => a,
        Err(_) => {
            temp_mount = true;
            state.fuse.dic.store.add_temp_point(&mono_path).await.unwrap()
        },
    };

    // return fail if this inode is mounted.
    if state.fuse.is_mount(inode).await{
        return axum::Json(MountResponse {
            status: FAIL.into(),
            mount: MountInfo::default(),
            message: "The target is mounted.".to_string(),
        })
    }

    let mut ml = state.manager.lock().await;
    if let Err(mounted_path) = ml.check_before_mount(&mono_path){
        return axum::Json(MountResponse {
            status: FAIL.into(),
            mount: MountInfo::default(),
            message: format!("The {} is already check-out ",mounted_path),
        })
    }
    let store_path = scorpio_config::store_path();
    // if it is a temp mount , mount it & return the hash and path.
    if temp_mount{
        let temp_hash = {
            let hasher = SHA1::new(mono_path.as_bytes());
            hasher.to_string()
        };

        let store_path = PathBuf::from(store_path).join(&temp_hash);
        let _ = state.fuse.overlay_mount(inode, store_path).await;
        let mount_info = MountInfo{
            hash: temp_hash.clone(),
            path: mono_path.clone(),
            inode,
        };
        ml.works.push(
            WorkDir{
                path: mono_path,
                node: inode,
                hash: temp_hash,
            }
        );
        let _ =  ml.to_toml("config.toml");
        return  axum::Json(MountResponse {
             status: SUCCESS.into(),
             mount: mount_info,
             message: "Directory mounted successfully".to_string(),
         })
    }
    // fetch the dionary node info from mono.
    let work_dir = fetch(&mut ml,inode, mono_path).await;
    let store_path = PathBuf::from(store_path).join(&work_dir.hash);
    // checkout / mount this dictionary. 
    
    let _ = state.fuse.overlay_mount(inode, store_path).await;
    
    let mount_info = MountInfo{
        hash:work_dir.hash,
        path: work_dir.path,
        inode,
    };
    axum::Json(MountResponse {
        status: SUCCESS.into(),
        mount: mount_info,
        message: "Directory mounted successfully".to_string(),
    })
}
/// Get all mounted dictionary . 
async fn mounts_handler(State(state): State<ScoState>) -> axum::Json<MountsResponse> {
    let manager = state.manager.lock().await;
    let re = manager.works.iter().map(|word_dir| MountInfo{
        hash: word_dir.hash.clone(),
        path: word_dir.path.clone(),
        inode: word_dir.node,
    }).collect();


    axum::Json(MountsResponse {
        status: SUCCESS.into(),
        mounts: re,
    })
}

async fn umount_handler(
    State(state): State<ScoState>,
    req: axum::Json<UmountRequest>,
) -> axum::Json<UmountResponse> {


    let handle;
    if let Some(inode) = req.inode{
        handle= state.fuse.overlay_umount_byinode(inode).await;
    }else if let Some(path) = &req.path{
        handle = state.fuse.overlay_umount_bypath(path).await;
    }else{
        return  axum::Json(UmountResponse {
            status: FAIL.into(),
            message: "Need a inode or path.".to_string(),
        })
    }
    match handle {
        Ok(_) => {
            if let Some(path) = &req.path{
                let _ = state.manager.lock().await.remove_workspace(path).await;
            }else{
                //todo be path by inode . 
                let path = state.fuse.dic.store.find_path(req.inode.unwrap()).await.unwrap();
                
                let _ = state.manager.lock().await.remove_workspace(&path.to_string()).await;

            }
           
            axum::Json(UmountResponse {
                status: SUCCESS.into(),
                message: "Directory unmounted successfully".to_string(),
            })
        },
        Err(err) => axum::Json(UmountResponse {
            status: FAIL.into(),
            message:format!("Umount process error :{}.",err),
        }),
    }
    
}

async fn config_handler() -> axum::Json<ConfigResponse> {
    let base_url = scorpio_config::base_url();
    let workspace = scorpio_config::workspace();
    let store_path = scorpio_config::store_path();
    let config_info = ConfigInfo {
        mega_url:base_url.to_string(),
        mount_path: workspace.to_string(),
        store_path: store_path.to_string(),
    };

    axum::Json(ConfigResponse {
        status: SUCCESS.into(),
        config: config_info,
    })
}

async fn update_config_handler(
    State(_state): State<ScoState>,
    req: axum::Json<ConfigRequest>,
) -> axum::Json<ConfigResponse> {
    // update the Configration by request.
    let config_info = ConfigInfo {
        mega_url: req.mega_url.clone().unwrap_or_default(),
        mount_path: req.mount_path.clone().unwrap_or_default(),
        store_path: req.store_path.clone().unwrap_or_default(),
    };

    axum::Json(ConfigResponse {
        status: "success".to_string(),
        config: config_info,
    })
}

