//! Antares filesystem management via scorpiofs direct calls.
//!
//! This module provides a singleton wrapper around `scorpiofs::AntaresManager`
//! for managing overlay filesystem mounts used during build operations.

use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use scorpiofs::{AntaresConfig, AntaresManager, AntaresPaths};
use tokio::sync::OnceCell;

static MANAGER: OnceCell<Arc<AntaresManager>> = OnceCell::const_new();
const TEST_BROWSE_JOB_ID: &str = "antares_test";

type DynError = Box<dyn Error + Send + Sync>;

/// Get the global AntaresManager instance.
///
/// Initializes the manager on first call by loading the scorpio configuration
/// from the path specified by `SCORPIO_CONFIG` environment variable.
///
/// If `SCORPIO_CONFIG` is not set, Orion will look for `scorpio.toml` in:
/// 1. Current working directory
/// 2. Next to the executable
/// 3. `/etc/scorpio/scorpio.toml` (system default)
///
/// Returns an error if no config file is found.
async fn get_manager() -> Result<&'static Arc<AntaresManager>, DynError> {
    MANAGER
        .get_or_try_init(|| async {
            let config_path = resolve_config_path()?;
            let config_path_str = config_path.to_str().ok_or_else(|| -> DynError {
                Box::new(io_other("Invalid SCORPIO_CONFIG path (non-UTF8)"))
            })?;

            tracing::info!("Initializing Antares with config: {}", config_path_str);

            scorpiofs::util::config::init_config(config_path_str).map_err(|e| {
                io_other(format!(
                    "Failed to load scorpio config from {config_path_str}: {e}. \
Hint: set SCORPIO_CONFIG=/path/to/scorpio.toml or create /etc/scorpio/scorpio.toml"
                ))
            })?;

            let paths = AntaresPaths::from_global_config();
            let manager = AntaresManager::new(paths).await;
            Ok(Arc::new(manager))
        })
        .await
}

fn io_other(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

fn resolve_config_path() -> Result<PathBuf, DynError> {
    // 1. Check SCORPIO_CONFIG environment variable
    if let Ok(path) = std::env::var("SCORPIO_CONFIG") {
        let config_path = PathBuf::from(&path);
        if config_path.exists() {
            return Ok(config_path);
        }
        return Err(Box::new(io_other(format!(
            "SCORPIO_CONFIG is set but file does not exist: {}",
            config_path.display()
        ))));
    }

    // 2. Check current working directory
    let cwd = std::env::current_dir().map_err(|e| {
        Box::new(io_other(format!(
            "Failed to get current working directory: {e}"
        ))) as DynError
    })?;
    let cwd_candidate = cwd.join("scorpio.toml");
    if cwd_candidate.exists() {
        return Ok(cwd_candidate);
    }

    // 3. Check next to executable
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let exe_candidate = exe_dir.join("scorpio.toml");
        if exe_candidate.exists() {
            return Ok(exe_candidate);
        }
    }

    // 4. Check system default path
    let system_candidate = PathBuf::from("/etc/scorpio/scorpio.toml");
    if system_candidate.exists() {
        return Ok(system_candidate);
    }

    Err(Box::new(io_other(format!(
        "Scorpio config not found. Set SCORPIO_CONFIG=/path/to/scorpio.toml, \
         place scorpio.toml in the working directory ({}), \
         or create /etc/scorpio/scorpio.toml",
        cwd.display()
    ))))
}

/// Mount a job overlay filesystem.
///
/// Creates a new Antares overlay mount for the specified job. The underlying
/// Dicfuse layer provides read-only access to the repository, while the overlay
/// allows copy-on-write modifications.
///
/// # Arguments
/// * `job_id` - Unique identifier for this build job
/// * `cl` - Optional changelist layer name
///
/// # Returns
/// The `AntaresConfig` containing mountpoint and job metadata on success.
pub async fn mount_job(job_id: &str, cl: Option<&str>) -> Result<AntaresConfig, DynError> {
    tracing::debug!("Mounting Antares job: job_id={}, cl={:?}", job_id, cl);
    get_manager()
        .await?
        .mount_job(job_id, cl)
        .await
        .map_err(Into::into)
}

/// Initialize Antares during Orion startup and eagerly trigger Dicfuse import.
///
/// This keeps the first build request from paying the full Dicfuse cold-start
/// cost. Readiness waiting runs in the background so Orion can continue booting.
#[allow(dead_code)] // Called from the bin target (main.rs), not visible to lib check.
pub(crate) async fn warmup_dicfuse() -> Result<(), DynError> {
    tracing::info!("Initializing Antares Dicfuse during Orion startup");
    let manager = get_manager().await?;
    let manager_for_test_mount = Arc::clone(manager);
    let dicfuse = manager.dicfuse();

    // Idempotent: safe even if the manager already started import internally.
    dicfuse.start_import();

    tokio::spawn(async move {
        let warmup_timeout_secs: u64 = std::env::var("ORION_DICFUSE_WARMUP_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1200);
        tracing::info!(
            "Waiting for Antares Dicfuse warmup to finish (timeout: {}s)",
            warmup_timeout_secs
        );

        match tokio::time::timeout(
            Duration::from_secs(warmup_timeout_secs),
            dicfuse.store.wait_for_ready(),
        )
        .await
        {
            Ok(_) => {
                tracing::info!("Antares Dicfuse warmup completed");
                log_dicfuse_root_tree();
                if is_test_mount_enabled() {
                    ensure_test_mount(manager_for_test_mount.as_ref()).await;
                } else {
                    tracing::info!(
                        "Antares test mount disabled by ORION_ENABLE_ANTARES_TEST_MOUNT"
                    );
                }
            }
            Err(_) => tracing::warn!(
                "Antares Dicfuse warmup timed out after {}s",
                warmup_timeout_secs
            ),
        }
    });

    Ok(())
}

fn is_test_mount_enabled() -> bool {
    match std::env::var("ORION_ENABLE_ANTARES_TEST_MOUNT") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            !(v == "0" || v == "false" || v == "no" || v == "off")
        }
        Err(_) => true,
    }
}

async fn ensure_test_mount(manager: &AntaresManager) {
    match manager.mount_job(TEST_BROWSE_JOB_ID, None).await {
        Ok(config) => {
            tracing::info!(
                "Antares test mount ready: job_id={}, mountpoint={}",
                TEST_BROWSE_JOB_ID,
                config.mountpoint.display()
            );
        }
        Err(err) => {
            tracing::warn!(
                "Failed to create Antares test mount job_id={}: {}",
                TEST_BROWSE_JOB_ID,
                err
            );
        }
    }
}

fn log_dicfuse_root_tree() {
    let root = PathBuf::from(scorpiofs::util::config::workspace());
    let max_depth = std::env::var("ORION_DICFUSE_ROOT_TREE_DEPTH")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(2);
    let max_entries = std::env::var("ORION_DICFUSE_ROOT_TREE_MAX_ENTRIES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(200);

    tracing::info!(
        root = %root.display(),
        max_depth,
        max_entries,
        "Dicfuse init: printing workspace root tree"
    );

    if !root.exists() {
        tracing::warn!("Dicfuse workspace path does not exist: {}", root.display());
        return;
    }

    let mut printed = 0usize;
    tracing::info!("[dicfuse-root] /");
    log_tree_recursive(&root, &root, 0, max_depth, max_entries, &mut printed);

    if printed >= max_entries {
        tracing::info!(
            "Dicfuse root tree output truncated at {} entries (set ORION_DICFUSE_ROOT_TREE_MAX_ENTRIES to increase)",
            max_entries
        );
    }
}

fn log_tree_recursive(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    printed: &mut usize,
) {
    if depth >= max_depth || *printed >= max_entries {
        return;
    }

    let entries = match std::fs::read_dir(current) {
        Ok(entries) => entries,
        Err(err) => {
            tracing::warn!("Failed to read {}: {}", current.display(), err);
            return;
        }
    };

    let mut children: Vec<(String, PathBuf, bool)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                tracing::warn!("read_dir entry error under {}: {}", current.display(), err);
                continue;
            }
        };

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        children.push((name, path, is_dir));
    }

    children.sort_by(|a, b| a.0.cmp(&b.0));

    for (_name, path, is_dir) in children {
        if *printed >= max_entries {
            return;
        }

        let rel = path
            .strip_prefix(root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        let indent = "  ".repeat(depth + 1);
        if is_dir {
            tracing::info!("[dicfuse-root] {}{}/", indent, rel);
        } else {
            tracing::info!("[dicfuse-root] {}{}", indent, rel);
        }
        *printed += 1;

        if is_dir {
            log_tree_recursive(root, &path, depth + 1, max_depth, max_entries, printed);
        }
    }
}

/// Unmount and cleanup a job overlay filesystem.
///
/// # Arguments
/// * `job_id` - The job identifier to unmount
///
/// # Returns
/// The `AntaresConfig` of the unmounted job if it existed.
#[allow(dead_code)]
pub async fn unmount_job(job_id: &str) -> Result<Option<AntaresConfig>, DynError> {
    tracing::debug!("Unmounting Antares job: job_id={}", job_id);
    get_manager()
        .await?
        .umount_job(job_id)
        .await
        .map_err(Into::into)
}
