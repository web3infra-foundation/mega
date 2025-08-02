use std::path::PathBuf;
use std::sync::Arc;

use crate::fuse::MegaFuse;
use crate::manager::fetch::fetch;
use crate::manager::{mr, ScorpioManager, WorkDir};
use crate::util::{config, GPath};
use axum::extract::State;
use axum::routing::{get, post};
use axum::Router;
use dashmap::DashMap;
use mercury::hash::SHA1;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;
mod git;
const SUCCESS: &str = "Success";
const FAIL: &str = "Fail";

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MountRequest {
    path: String,
    mr: Option<String>, // mr is the mount request, used for buck2 temp mount.
}

#[derive(Debug, Deserialize, Serialize)]
struct MountResponse {
    status: String,
    mount: MountInfo,
    message: String,
}
/// Orion mount task structure, used to track asynchronous mount operations.
/// Each task represents a mount request that can be executed in the background.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct OrionMount {
    request_id: String,        // Unique identifier for the mount request
    status: String,            // Current task status: "fetching", "finished", or "error"
    mount_info: MountRequest,  // Original mount request containing path and mr info
    result: Option<MountInfo>, // Mount result populated when task completes successfully
}
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
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

/// Response structure for orion mount requests.
/// Returns immediately with a request ID for tracking the async operation.
#[derive(Debug, Deserialize, Serialize)]
struct OrionMountResponse {
    status: String,     // Operation status: "Success" or "Fail"
    request_id: String, // Unique ID for tracking the mount task
    message: String,    // Human-readable status message
}

/// Request structure for querying orion mount task status.
/// Supports lookup by either request_id or path.
#[derive(Debug, Deserialize, Serialize)]
struct OrionSelectRequest {
    request_id: Option<String>, // Optional: unique task identifier
    path: Option<String>,       // Optional: mount path for task lookup
}

/// Response structure for orion mount task status queries.
/// Provides current task status and mount information when available.
#[derive(Debug, Deserialize, Serialize)]
struct OrionSelectResponse {
    status: String,           // API call status: "Success" or "Fail"
    task_status: String,      // Task status: "fetching", "finished", "error", or "not_found"
    mount: Option<MountInfo>, // Mount information available when task is finished
    message: String,          // Human-readable status message
}
/// Application state shared across all request handlers.
/// Contains shared resources and task tracking for the daemon.
#[derive(Clone)]
struct ScoState {
    fuse: Arc<MegaFuse>,                           // Shared FUSE filesystem interface
    manager: Arc<Mutex<ScorpioManager>>,           // Shared workspace manager
    orion_tasks: Arc<DashMap<String, OrionMount>>, // Thread-safe storage for async mount tasks
}
#[allow(unused)]
pub async fn daemon_main(fuse: Arc<MegaFuse>, manager: ScorpioManager) {
    let inner = ScoState {
        fuse,
        manager: Arc::new(Mutex::new(manager)),
        orion_tasks: Arc::new(DashMap::new()), // Initialize empty task tracking map
    };
    let mut app = Router::new()
        .route("/api/fs/mount", post(mount_handler))
        .route("/api/fs/mpoint", get(mounts_handler))
        .route("/api/fs/orion_mount", post(orion_mount_handler))
        .route("/api/fs/orion_select", post(orion_select_handler))
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
    let inode = match state.fuse.get_inode(&mono_path).await {
        Ok(a) => a,
        Err(_) => {
            temp_mount = true;
            state
                .fuse
                .dic
                .store
                .add_temp_point(&mono_path)
                .await
                .unwrap()
        }
    };

    // return fail if this inode is mounted.
    if state.fuse.is_mount(inode).await {
        return axum::Json(MountResponse {
            status: FAIL.into(),
            mount: MountInfo::default(),
            message: "The target is mounted.".to_string(),
        });
    }

    let mut ml = state.manager.lock().await;
    if let Err(mounted_path) = ml.check_before_mount(&mono_path) {
        return axum::Json(MountResponse {
            status: FAIL.into(),
            mount: MountInfo::default(),
            message: format!("The {mounted_path} is already check-out "),
        });
    }
    let store_path = config::store_path();
    // if it is a temp mount , mount it & return the hash and path.
    if temp_mount {
        let temp_hash = {
            let hasher = SHA1::new(mono_path.as_bytes());
            hasher.to_string()
        };

        let store_path = PathBuf::from(store_path).join(&temp_hash);
        let _ = state.fuse.overlay_mount(inode, store_path, false).await;
        let mount_info = MountInfo {
            hash: temp_hash.clone(),
            path: mono_path.clone(),
            inode,
        };
        ml.works.push(WorkDir {
            path: mono_path,
            node: inode,
            hash: temp_hash,
        });
        let _ = ml.to_toml("config.toml");
        return axum::Json(MountResponse {
            status: SUCCESS.into(),
            mount: mount_info,
            message: "Directory mounted successfully".to_string(),
        });
    }

    // fetch the dictionary node info from mono.
    let work_dir = fetch(&mut ml, inode, mono_path).await.unwrap();
    let store_path = PathBuf::from(store_path).join(&work_dir.hash);
    if let Some(m) = &req.mr {
        let mr_store_path = PathBuf::from(&store_path).join("mr");
        // if mr is provided, we need to fetch the mr info from mono.
        if let Err(e) = mr::build_mr_layer(m, mr_store_path).await {
            return axum::Json(MountResponse {
                status: FAIL.into(),
                mount: MountInfo::default(),
                message: format!("Failed to build mr layer: {e}"),
            });
        }
    }

    // checkout / mount this dictionary.
    if let Err(e) = state
        .fuse
        .overlay_mount(inode, store_path, req.mr.is_some())
        .await
    {
        return axum::Json(MountResponse {
            status: FAIL.into(),
            mount: MountInfo::default(),
            message: format!("Mount process error: {e}."),
        });
    }

    let mount_info = MountInfo {
        hash: work_dir.hash,
        path: work_dir.path,
        inode,
    };
    axum::Json(MountResponse {
        status: SUCCESS.into(),
        mount: mount_info,
        message: "Directory mounted successfully".to_string(),
    })
}

/// Asynchronous mount handler for Orion clients.
/// Initiates a mount operation in the background and returns immediately with a tracking ID.
/// This allows clients to start multiple mount operations concurrently and check their status.
async fn orion_mount_handler(
    State(state): State<ScoState>,
    req: axum::Json<MountRequest>,
) -> axum::Json<OrionMountResponse> {
    // Generate a unique request ID for tracking this mount operation
    let request_id = Uuid::new_v4().to_string();

    // Create initial task record with "fetching" status
    let orion_mount = OrionMount {
        request_id: request_id.clone(),
        status: "fetching".to_string(),
        mount_info: req.0.clone(),
        result: None,
    };

    // Store the task in the shared task map for status tracking
    state.orion_tasks.insert(request_id.clone(), orion_mount);

    // Spawn background task to perform the actual mount operation
    let state_clone = state.clone();
    let req_clone = req.0.clone();
    let request_id_clone = request_id.clone();

    tokio::spawn(async move {
        // Clone state to avoid ownership issues in the async task
        let state_for_task = state_clone.clone();
        let mount_result = perform_mount_task(state_for_task, req_clone).await;

        // Update the task status based on mount operation result
        if let Some(mut task) = state_clone.orion_tasks.get_mut(&request_id_clone) {
            match mount_result {
                Ok(mount_info) => {
                    task.status = "finished".to_string();
                    task.result = Some(mount_info);
                }
                Err(_) => {
                    task.status = "error".to_string();
                    // Could add error details here in the future
                }
            }
        }
    });

    // Return immediately with the request ID for client tracking
    axum::Json(OrionMountResponse {
        status: SUCCESS.to_string(),
        request_id,
        message: "Mount task started successfully".to_string(),
    })
}

/// Helper function to perform the actual mount operation.
/// This function contains the core mounting logic extracted from the original mount handler.
/// It handles both temporary mounts (for buck2) and regular mounts with proper error handling.
async fn perform_mount_task(state: ScoState, req: MountRequest) -> Result<MountInfo, String> {
    // Normalize the path format using GPath utility
    let mono_path = GPath::from(req.path.clone()).to_string();

    // Determine if this is a temporary mount for buck2 workflow
    let mut temp_mount = false;

    // Try to get existing inode, or create temporary point if path doesn't exist
    let inode = match state.fuse.get_inode(&mono_path).await {
        Ok(a) => a,
        Err(_) => {
            temp_mount = true;
            state
                .fuse
                .dic
                .store
                .add_temp_point(&mono_path)
                .await
                .map_err(|e| format!("Failed to add temp point: {e}"))?
        }
    };

    // Check if the target is already mounted to prevent conflicts
    if state.fuse.is_mount(inode).await {
        return Err("The target is already mounted".to_string());
    }

    // Acquire manager lock and check for existing checkouts
    let mut ml = state.manager.lock().await;
    if let Err(mounted_path) = ml.check_before_mount(&mono_path) {
        return Err(format!("The {mounted_path} is already check-out"));
    }

    let store_path = config::store_path();

    // Handle temporary mount case (typically for buck2)
    if temp_mount {
        let temp_hash = {
            let hasher = SHA1::new(mono_path.as_bytes());
            hasher.to_string()
        };

        let store_path = PathBuf::from(store_path).join(&temp_hash);

        // Perform the actual overlay mount
        state
            .fuse
            .overlay_mount(inode, store_path, false)
            .await
            .map_err(|e| format!("Failed to overlay mount: {e}"))?;

        let mount_info = MountInfo {
            hash: temp_hash.clone(),
            path: mono_path.clone(),
            inode,
        };

        // Update manager's work directory list
        ml.works.push(WorkDir {
            path: mono_path,
            node: inode,
            hash: temp_hash,
        });
        let _ = ml.to_toml("config.toml");

        return Ok(mount_info);
    }

    // Handle regular mount case - fetch repository information
    let work_dir = fetch(&mut ml, inode, mono_path)
        .await
        .map_err(|e| format!("Failed to fetch: {e}"))?;
    let store_path = PathBuf::from(store_path).join(&work_dir.hash);

    // Handle merge request (MR) layer if provided
    if let Some(m) = &req.mr {
        let mr_store_path = PathBuf::from(&store_path).join("mr");
        if let Err(e) = mr::build_mr_layer(m, mr_store_path).await {
            return Err(format!("Failed to build mr layer: {e}"));
        }
    }

    // Perform the final overlay mount with MR layer if applicable
    state
        .fuse
        .overlay_mount(inode, store_path, req.mr.is_some())
        .await
        .map_err(|e| format!("Mount process error: {e}"))?;

    let mount_info = MountInfo {
        hash: work_dir.hash,
        path: work_dir.path,
        inode,
    };

    Ok(mount_info)
}

/// Query handler for Orion mount task status.
/// Allows clients to check the progress of their asynchronous mount operations.
/// Supports lookup by either request_id (preferred) or path.
async fn orion_select_handler(
    State(state): State<ScoState>,
    req: axum::Json<OrionSelectRequest>,
) -> axum::Json<OrionSelectResponse> {
    // Primary lookup method: search by request_id
    if let Some(request_id) = &req.request_id {
        if let Some(task) = state.orion_tasks.get(request_id) {
            return axum::Json(OrionSelectResponse {
                status: SUCCESS.to_string(),
                task_status: task.status.clone(),
                mount: task.result.clone(),
                message: "Task found".to_string(),
            });
        } else {
            return axum::Json(OrionSelectResponse {
                status: FAIL.to_string(),
                task_status: "not_found".to_string(),
                mount: None,
                message: "Task not found".to_string(),
            });
        }
    }

    // Secondary lookup method: search by path
    // This is less efficient as it requires iterating through all tasks
    if let Some(path) = &req.path {
        let mono_path = GPath::from(path.clone()).to_string();

        // Search through all active tasks to find one with matching path
        for task_ref in state.orion_tasks.iter() {
            let task = task_ref.value();
            if task.mount_info.path == mono_path {
                return axum::Json(OrionSelectResponse {
                    status: SUCCESS.to_string(),
                    task_status: task.status.clone(),
                    mount: task.result.clone(),
                    message: "Task found by path".to_string(),
                });
            }
        }

        return axum::Json(OrionSelectResponse {
            status: FAIL.to_string(),
            task_status: "not_found".to_string(),
            mount: None,
            message: "No task found for this path".to_string(),
        });
    }

    // Invalid request: neither request_id nor path provided
    // TODO: Consider implementing cleanup for completed tasks to prevent memory leaks
    axum::Json(OrionSelectResponse {
        status: FAIL.to_string(),
        task_status: "invalid_request".to_string(),
        mount: None,
        message: "Either request_id or path must be provided".to_string(),
    })
}

/// Get all mounted dictionary .
async fn mounts_handler(State(state): State<ScoState>) -> axum::Json<MountsResponse> {
    let manager = state.manager.lock().await;
    let re = manager
        .works
        .iter()
        .map(|word_dir| MountInfo {
            hash: word_dir.hash.clone(),
            path: word_dir.path.clone(),
            inode: word_dir.node,
        })
        .collect();

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
    if let Some(inode) = req.inode {
        handle = state.fuse.overlay_umount_byinode(inode).await;
    } else if let Some(path) = &req.path {
        handle = state.fuse.overlay_umount_bypath(path).await;
    } else {
        return axum::Json(UmountResponse {
            status: FAIL.into(),
            message: "Need a inode or path.".to_string(),
        });
    }
    match handle {
        Ok(_) => {
            if let Some(path) = &req.path {
                let _ = state.manager.lock().await.remove_workspace(path).await;
            } else {
                //todo be path by inode .
                let path = state
                    .fuse
                    .dic
                    .store
                    .find_path(req.inode.unwrap())
                    .await
                    .unwrap();

                let _ = state
                    .manager
                    .lock()
                    .await
                    .remove_workspace(&path.to_string())
                    .await;
            }

            axum::Json(UmountResponse {
                status: SUCCESS.into(),
                message: "Directory unmounted successfully".to_string(),
            })
        }
        Err(err) => axum::Json(UmountResponse {
            status: FAIL.into(),
            message: format!("Umount process error :{err}."),
        }),
    }
}

async fn config_handler() -> axum::Json<ConfigResponse> {
    let base_url = config::base_url();
    let workspace = config::workspace();
    let store_path = config::store_path();
    let config_info = ConfigInfo {
        mega_url: base_url.to_string(),
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
