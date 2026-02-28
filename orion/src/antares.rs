//! Antares filesystem management via scorpiofs direct calls.
//!
//! This module provides a singleton wrapper around `scorpiofs::AntaresManager`
//! for managing overlay filesystem mounts used during build operations.

use std::{error::Error, io, path::PathBuf, sync::Arc};

use scorpiofs::{AntaresConfig, AntaresManager, AntaresPaths};
use tokio::sync::OnceCell;

static MANAGER: OnceCell<Arc<AntaresManager>> = OnceCell::const_new();

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

/// Unmount and cleanup a job overlay filesystem.
///
/// # Arguments
/// * `job_id` - The job identifier to unmount
///
/// # Returns
/// The `AntaresConfig` of the unmounted job if it existed.
pub async fn unmount_job(job_id: &str) -> Result<Option<AntaresConfig>, DynError> {
    tracing::debug!("Unmounting Antares job: job_id={}", job_id);
    get_manager()
        .await?
        .umount_job(job_id)
        .await
        .map_err(Into::into)
}
