use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Router,
};
use dashmap::DashMap;
use git_internal::hash::ObjectHash;
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::{
    fuse::MegaFuse,
    manager::{fetch::fetch, ScorpioManager, WorkDir},
    util::{config, GPath},
};
pub mod antares;
//mod git;

const SUCCESS: &str = "Success";
const FAIL: &str = "Fail";

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MountRequest {
    path: String,
    cl: Option<String>, // cl is the mount request, used for buck2 temp mount.
}

/// Response structure for mount requests.
/// Returns immediately with a request ID for tracking the async operation.
#[derive(Debug, Deserialize, Serialize)]
struct MountResponse {
    status: String,     // Operation status: "Success" or "Fail"
    request_id: String, // Unique ID for tracking the mount task
    message: String,    // Human-readable status message
}
/// Mount task structure, used to track asynchronous mount operations.
/// Each task represents a mount request that can be executed in the background.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct MountStatus {
    request_id: String,        // Unique identifier for the mount request
    status: String,            // Current task status: "fetching", "finished", or "error"
    mount_info: MountRequest,  // Original mount request containing path and cl info
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

/// Response structure for mount task status queries.
/// Provides current task status and mount information when available.
#[derive(Debug, Deserialize, Serialize)]
struct SelectResponse {
    status: String,           // API call status: "Success" or "Fail"
    task_status: String,      // Task status: "fetching", "finished", "error", or "not_found"
    mount: Option<MountInfo>, // Mount information available when task is finished
    message: String,          // Human-readable status message
}
/// Application state shared across all request handlers.
/// Contains shared resources and task tracking for the daemon.
#[derive(Clone)]
struct ScoState {
    fuse: Arc<MegaFuse>,                      // Shared FUSE filesystem interface
    manager: Arc<Mutex<ScorpioManager>>,      // Shared workspace manager
    tasks: Arc<DashMap<String, MountStatus>>, // Thread-safe storage for async mount tasks
}

/// Resolve a mount request to an inode and whether it should be treated as a temporary mount.
///
/// - If the path exists, returns its inode and `temp_mount=false`.
/// - If the path doesn't exist and this is a temp mount request (buck2), creates a temp point.
/// - If the path doesn't exist and this is a normal mount, returns a descriptive error.
async fn resolve_mount_inode(
    state: &ScoState,
    req: &MountRequest,
    mono_path: &str,
) -> Result<(u64, bool), String> {
    let temp_request = req.cl.is_none();

    match state.fuse.get_inode(mono_path).await {
        Ok(inode) => Ok((inode, false)),
        Err(_) => {
            if temp_request {
                let inode = match state.fuse.dic.store.add_temp_point(mono_path).await {
                    Ok(inode) => inode,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            if let Ok(crate::dicfuse::store::PathLookupStatus::ParentNotLoaded {
                                parent_path,
                            }) = state.fuse.dic.store.lookup_path_status(mono_path).await
                            {
                                return Err(format!(
                                    "Temp mount parent directory not loaded in dicfuse: {mono_path} (parent: {parent_path})"
                                ));
                            }
                        }
                        return Err(format!("Failed to add temp point for {mono_path}: {e}"));
                    }
                };
                return Ok((inode, true));
            }

            let status = state
                .fuse
                .dic
                .store
                .lookup_path_status(mono_path)
                .await
                .map_err(|e| format!("Mount path lookup failed in dicfuse: {mono_path}: {e}"))?;

            match status {
                crate::dicfuse::store::PathLookupStatus::Found(inode) => Ok((inode, false)),
                crate::dicfuse::store::PathLookupStatus::ParentNotLoaded { parent_path } => Err(
                    format!(
                        "Mount parent directory not loaded in dicfuse: {mono_path} (parent: {parent_path})"
                    ),
                ),
                crate::dicfuse::store::PathLookupStatus::NotFound => Err(format!(
                    "Mount path not found in dicfuse: {mono_path}"
                )),
            }
        }
    }
}
#[allow(unused)]
pub async fn daemon_main(
    fuse: Arc<MegaFuse>,
    manager: ScorpioManager,
    shutdown_rx: oneshot::Receiver<()>,
    bind_addr: SocketAddr,
) {
    let inner = ScoState {
        fuse,
        manager: Arc::new(Mutex::new(manager)),
        tasks: Arc::new(DashMap::new()), // Initialize empty task tracking map
    };
    let mut app = Router::new()
        .route("/api/fs/mount", post(mount_handler))
        .route("/api/fs/mpoint", get(mounts_handler))
        .route("/api/fs/select/{request_id}", get(select_handler))
        .route("/api/fs/unmount", post(unmount_handler))
        .route("/api/config", get(config_handler))
        .route("/api/config", post(update_config_handler))
        // Note: git-related routes have been moved to `src/daemon/git.rs`
        // and are currently disabled here. To enable them, merge the
        // router returned by `daemon::git::router()` into this `app`.
        .with_state(inner);

    // Antares route - create service with new Dicfuse instance
    let antares_service = Arc::new(antares::AntaresServiceImpl::new(None).await);
    let antares_service_for_shutdown = antares_service.clone();
    let antares_daemon = antares::AntaresDaemon::new(antares_service);
    let antares_router = antares_daemon.router();
    let app = app.nest("/antares", antares_router);

    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
            tracing::info!("HTTP server shutdown requested; running Antares shutdown cleanup");
            match tokio::time::timeout(
                std::time::Duration::from_secs(15),
                antares_service_for_shutdown.shutdown_cleanup_impl(),
            )
            .await
            {
                Ok(Ok(())) => tracing::info!("Antares shutdown cleanup completed"),
                Ok(Err(e)) => tracing::warn!("Antares shutdown cleanup failed: {:?}", e),
                Err(_) => tracing::warn!("Antares shutdown cleanup timed out"),
            }
        })
        .await
        .unwrap()
}

/// Asynchronous mount handler for clients.
/// Initiates a mount operation in the background and returns immediately with a tracking ID.
/// This allows clients to start multiple mount operations concurrently and check their status.
async fn mount_handler(
    State(state): State<ScoState>,
    req: axum::Json<MountRequest>,
) -> axum::Json<MountResponse> {
    // Generate a unique request ID for tracking this mount operation
    let request_id = Uuid::new_v4().to_string();

    // Create initial task record with "fetching" status
    let mount_status = MountStatus {
        request_id: request_id.clone(),
        status: "fetching".to_string(),
        mount_info: req.0.clone(),
        result: None,
    };

    // Store the task in the shared task map for status tracking
    state.tasks.insert(request_id.clone(), mount_status);

    // Perform the mount operation synchronously
    let mount_result = perform_mount_task(state.clone(), req.0.clone()).await;

    // Update the task status based on mount operation result
    if let Some(mut task) = state.tasks.get_mut(&request_id) {
        match mount_result {
            Ok(mount_info) => {
                task.status = "finished".to_string();
                task.result = Some(mount_info);
                axum::Json(MountResponse {
                    status: SUCCESS.to_string(),
                    request_id,
                    message: "Mount task completed".to_string(),
                })
            }
            Err(err) => {
                task.status = "error".to_string();
                let message =
                    if err.contains("already mounted") || err.contains("already checked-out") {
                        "please unmount".to_string()
                    } else {
                        format!("Mount failed: {}", err)
                    };

                axum::Json(MountResponse {
                    status: FAIL.to_string(),
                    request_id,
                    message,
                })
            }
        }
    } else {
        axum::Json(MountResponse {
            status: FAIL.to_string(),
            request_id,
            message: "task not found".to_string(),
        })
    }
}

/// Helper function to perform the actual mount operation.
/// This function contains the core mounting logic extracted from the original mount handler.
/// It handles both temporary mounts (for buck2) and regular mounts with proper error handling.
async fn perform_mount_task(state: ScoState, req: MountRequest) -> Result<MountInfo, String> {
    // Normalize the path format using GPath utility
    let mono_path = if let Some(cl) = &req.cl {
        format!("{}_{}", GPath::from(req.path.clone()), cl)
    } else {
        GPath::from(req.path.clone()).to_string()
    };

    // Resolve inode and determine temp mount behavior
    let (inode, temp_mount) = resolve_mount_inode(&state, &req, &mono_path).await?;

    // Check if the target is already mounted to prevent conflicts
    if state.fuse.is_mount(inode).await {
        return Err("The target is already mounted".to_string());
    }

    // Acquire manager lock and check for existing checkouts
    let mut ml = state.manager.lock().await;
    if let Err(mounted_path) = ml.check_before_mount(&mono_path) {
        return Err(format!("The {mounted_path} is already checked-out"));
    }

    let store_path = config::store_path();

    // Handle temporary mount case (typically for buck2)
    if temp_mount {
        let temp_hash = {
            let hasher = ObjectHash::new(mono_path.as_bytes());
            hasher.to_string()
        };

        let store_path = PathBuf::from(store_path).join(&temp_hash);

        // Perform the actual overlay mount
        state
            .fuse
            .overlay_mount(inode, store_path, false, None)
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
    let work_dir = fetch(&mut ml, inode, mono_path.clone(), &req.path)
        .await
        .map_err(|e| format!("Failed to fetch: {e}"))?;

    let store_path = PathBuf::from(store_path).join(&work_dir.hash);

    // CL layer support removed: skip building CL layer if provided

    // Perform the final overlay mount with CL layer if applicable
    state
        .fuse
        .overlay_mount(inode, store_path, req.cl.is_some(), req.cl.as_deref())
        .await
        .map_err(|e| format!("Mount process error: {e}"))?;

    let mount_info = MountInfo {
        hash: work_dir.hash,
        path: work_dir.path,
        inode,
    };

    Ok(mount_info)
}

/// Query handler for mount task status.
/// Allows clients to check the progress of their asynchronous mount operations.
/// Requires a valid request_id as URL path parameter.
/// Automatically cleans up completed tasks from memory.
async fn select_handler(
    State(state): State<ScoState>,
    Path(request_id): Path<String>,
) -> axum::Json<SelectResponse> {
    // Search by request_id (now provided as URL path parameter)
    if let Some(task) = state.tasks.get(&request_id) {
        let response = SelectResponse {
            status: SUCCESS.to_string(),
            task_status: task.status.clone(),
            mount: task.result.clone(),
            message: "Task found".to_string(),
        };

        // Clean up completed tasks from memory to prevent memory leaks
        if task.status == "finished" || task.status == "error" {
            drop(task); // Release the reference before removing
            state.tasks.remove(&request_id);
        }

        axum::Json(response)
    } else {
        axum::Json(SelectResponse {
            status: FAIL.to_string(),
            task_status: "not_found".to_string(),
            mount: None,
            message: "Task not found".to_string(),
        })
    }
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

/// Unmounts filesystem and removes CL layer files
async fn unmount_handler(
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
            let path_str = if let Some(path) = &req.path {
                path.clone()
            } else {
                //todo be path by inode .
                let path = state
                    .fuse
                    .dic
                    .store
                    .find_path(req.inode.unwrap())
                    .await
                    .unwrap();
                path.to_string()
            };

            // Try to get the CL link from the path and clean up CL layer
            if let Some(cl_pos) = path_str.rfind('_') {
                let potential_cl_link = &path_str[cl_pos + 1..];
                // Simple validation - CL links are usually not entire paths
                if !potential_cl_link.contains('/') && !potential_cl_link.is_empty() {
                    let store_path = config::store_path();
                    let _ = state
                        .fuse
                        .remove_cl_layer_by_cl_link(store_path, potential_cl_link)
                        .await;
                }
            }

            let _ = state.manager.lock().await.remove_workspace(&path_str).await;

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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::dicfuse::Dicfuse;

    async fn make_state() -> ScoState {
        let tmp = tempdir().unwrap();
        let dic = Dicfuse::new_with_store_path(tmp.path().to_str().unwrap()).await;
        let mut fuse = MegaFuse::new().await;
        fuse.dic = Arc::new(dic);

        ScoState {
            fuse: Arc::new(fuse),
            manager: Arc::new(Mutex::new(ScorpioManager { works: vec![] })),
            tasks: Arc::new(DashMap::new()),
        }
    }

    #[tokio::test]
    async fn test_resolve_mount_inode_found_path() {
        let state = make_state().await;
        state.fuse.dic.store.insert_mock_item(1, 0, "", true).await;
        state
            .fuse
            .dic
            .store
            .insert_mock_item(2, 1, "repo", true)
            .await;

        let req = MountRequest {
            path: "/repo".to_string(),
            cl: None,
        };
        let (inode, temp_mount) = resolve_mount_inode(&state, &req, "repo").await.unwrap();
        assert_eq!(inode, 2);
        assert!(!temp_mount);
    }

    #[tokio::test]
    async fn test_resolve_mount_inode_temp_mount() {
        let state = make_state().await;
        state.fuse.dic.store.insert_mock_item(1, 0, "", true).await;
        state
            .fuse
            .dic
            .store
            .insert_mock_item(2, 1, "repo", true)
            .await;

        let req = MountRequest {
            path: "/repo/tmp".to_string(),
            cl: None,
        };
        let (inode, temp_mount) = resolve_mount_inode(&state, &req, "repo/tmp").await.unwrap();
        assert!(temp_mount);
        let inode_check = state
            .fuse
            .dic
            .store
            .get_inode_from_path("repo/tmp")
            .await
            .unwrap();
        assert_eq!(inode, inode_check);
    }

    #[tokio::test]
    async fn test_resolve_mount_inode_not_found_normal_mount() {
        let state = make_state().await;
        state.fuse.dic.store.insert_mock_item(1, 0, "", true).await;
        state
            .fuse
            .dic
            .store
            .insert_mock_item(2, 1, "repo", true)
            .await;

        let req = MountRequest {
            path: "/repo/missing".to_string(),
            cl: Some("cl123".to_string()),
        };
        let err = resolve_mount_inode(&state, &req, "repo/missing_cl123")
            .await
            .unwrap_err();
        assert!(err.contains("Mount path not found in dicfuse"));
        assert!(err.contains("repo/missing_cl123"));
    }
}
