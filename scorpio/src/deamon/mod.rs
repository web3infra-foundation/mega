use axum::Router;
use axum::routing::{post, get};
use serde::{Deserialize, Serialize};


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

#[allow(unused)]
pub async fn deamon_main() {
    let app = Router::new()
        .route("/api/fs/mount", post(mount_handler))
        .route("/api/fs/mpoint", get(mounts_handler))
        .route("/api/fs/umount", post(umount_handler))
        .route("/api/config", get(config_handler))
        .route("/api/config", post(update_config_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:2725").await.unwrap();
   axum::serve(listener, app).await.unwrap()
}

async fn mount_handler(_req: axum::Json<MountRequest>) -> axum::Json<MountResponse> {
    // 在这里处理挂载文件目录的逻辑，返回相应的 MountResponse 结构体
    let mount_info = MountInfo {
        hash: "abc123".to_string(),
        path: "/mnt/mydir".to_string(),
        inode: 1001,
    };

    axum::Json(MountResponse {
        status: "success".to_string(),
        mount: mount_info,
        message: "Directory mounted successfully".to_string(),
    })
}

async fn mounts_handler() -> axum::Json<MountsResponse> {
    // 在这里处理获取挂载点目录的逻辑，返回相应的 MountsResponse 结构体
    let mount_info1 = MountInfo {
        hash: "abc123".to_string(),
        path: "/mnt/dir1".to_string(),
        inode: 1001,
    };

    let mount_info2 = MountInfo {
        hash: "def456".to_string(),
        path: "/mnt/dir2".to_string(),
        inode: 1002,
    };

    let mount_info3 = MountInfo {
        hash: "ghi789".to_string(),
        path: "/mnt/dir3".to_string(),
        inode: 1003,
    };

    let mounts = vec![mount_info1, mount_info2, mount_info3];

    axum::Json(MountsResponse {
        status: "success".to_string(),
        mounts,
    })
}

async fn umount_handler(_req: axum::Json<UmountRequest>) -> axum::Json<UmountResponse> {
    // 在这里处理卸载文件目录的逻辑，返回相应的 UmountResponse 结构体
    axum::Json(UmountResponse {
        status: "success".to_string(),
        message: "Directory unmounted successfully".to_string(),
    })
}

async fn config_handler() -> axum::Json<ConfigResponse> {
    // 在这里处理获取配置信息的逻辑，返回相应的 ConfigResponse 结构体
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

async fn update_config_handler(_req: axum::Json<ConfigRequest>) -> axum::Json<ConfigResponse> {
    // 在这里处理修改配置信息的逻辑，返回相应的 ConfigResponse 结构体
    axum::Json(ConfigResponse {
        status: "success".to_string(),
        config: ConfigInfo {
            mega_url: _req.mega_url.clone().unwrap_or_default(),
            mount_path: _req.mount_path.clone().unwrap_or_default(),
            store_path: _req.store_path.clone().unwrap_or_default(),
        },
    })
}