use std::sync::Arc;

use axum::extract::State;
use axum::Router;
use axum::routing::{post, get};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use crate::fuse::MegaFuse;
use crate::manager::ScorpioManager;


#[derive(Debug, Deserialize, Serialize)]
struct MountRequest {
    path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MountResponse {
    status: String,
    mount: MountInfo,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
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
pub async fn deamon_main(fuse:Arc<MegaFuse>,manager:ScorpioManager) {
    let inner = ScoState{
        fuse,
        manager: Arc::new(Mutex::new(manager)),
    };
    let app = Router::new()
        .route("/api/fs/mount", post(mount_handler))
        .route("/api/fs/mpoint", get(mounts_handler))
        .route("/api/fs/umount", post(umount_handler))
        .route("/api/config", get(config_handler))
        .route("/api/config", post(update_config_handler))
        .with_state(inner);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:2725").await.unwrap();
    axum::serve(listener, app).await.unwrap()
}

async fn mount_handler(
    State(state): State<ScoState>, // 注入共享状态
    req: axum::Json<MountRequest>,
) -> axum::Json<MountResponse> {
    // let mono_path = req.path.clone();
    // let inode = state.fuse.get_inode(&mono_path);
    // let mut ml = state.manager.lock().await;
    // let store_path = ml.store_path.clone();
    // let work_dir = fetch(&mut ml,inode, mono_path).await;
    // let store_path = PathBuf::from(store_path).join(&work_dir.hash);
    // state.fuse.overlay_mount(inode, store_path);
    // state.fuse.async_init(fuse_backend_rs::abi::fuse_abi::FsOptions::empty()).await;
    //let _ = state.fuse.init(fuse_backend_rs::abi::fuse_abi::FsOptions::empty());
    // 在这里可以访问 state.fuse 或 state.manager
    let mount_info = MountInfo{
        hash: "hash".to_string(),
        path: "path".to_string(),
        inode: 0,
    };

    axum::Json(MountResponse {
        status: "success".to_string(),
        mount: mount_info,
        message: "Directory mounted successfully".to_string(),
    })
}

async fn mounts_handler(State(_state): State<ScoState>) -> axum::Json<MountsResponse> {
    // 你可以在这里使用 state.fuse 或 state.manager 来获取挂载点信息
    let mounts = vec![
        MountInfo {
            hash: "abc123".to_string(),
            path: "/mnt/dir1".to_string(),
            inode: 1001,
        },
        MountInfo {
            hash: "def456".to_string(),
            path: "/mnt/dir2".to_string(),
            inode: 1002,
        },
        MountInfo {
            hash: "ghi789".to_string(),
            path: "/mnt/dir3".to_string(),
            inode: 1003,
        },
    ];

    axum::Json(MountsResponse {
        status: "success".to_string(),
        mounts,
    })
}

async fn umount_handler(
    State(_state): State<ScoState>,
    _req: axum::Json<UmountRequest>,
) -> axum::Json<UmountResponse> {
    // 在这里访问 state 进行卸载逻辑
    axum::Json(UmountResponse {
        status: "success".to_string(),
        message: "Directory unmounted successfully".to_string(),
    })
}

async fn config_handler(State(_state): State<ScoState>) -> axum::Json<ConfigResponse> {
    // 使用 state 访问配置逻辑
    let config_info = ConfigInfo {
        mega_url: "http://localhost:8000".to_string(),
        mount_path: "/home/luxian/megadir/mount".to_string(),
        store_path: "/home/luxian/megadir/store".to_string(),
    };

    axum::Json(ConfigResponse {
        status: "success".to_string(),
        config: config_info,
    })
}

async fn update_config_handler(
    State(_state): State<ScoState>,
    req: axum::Json<ConfigRequest>,
) -> axum::Json<ConfigResponse> {
    // 根据请求更新配置
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