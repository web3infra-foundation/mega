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
/// If `SCORPIO_CONFIG` is not set, Orion will look for `scorpio.toml` in the
/// process working directory, then next to the running executable.
///
/// If no config file is found, Orion will panic (fail-fast).
async fn get_manager() -> Result<&'static Arc<AntaresManager>, DynError> {
    MANAGER
        .get_or_try_init(|| async {
            let config_path = resolve_config_path();
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

fn resolve_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("SCORPIO_CONFIG") {
        let config_path = PathBuf::from(path);
        if config_path.exists() {
            return config_path;
        }

        panic!(
            "SCORPIO_CONFIG is set but file does not exist: {}",
            config_path.display()
        );
    }

    let cwd = std::env::current_dir().expect("Failed to get current working directory");
    let cwd_candidate = cwd.join("scorpio.toml");
    if cwd_candidate.exists() {
        return cwd_candidate;
    }

    let exe_candidate = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("scorpio.toml")));
    if let Some(exe_candidate) = exe_candidate
        && exe_candidate.exists()
    {
        return exe_candidate;
    }

    panic!(
        "Scorpio config not found. Set SCORPIO_CONFIG=/path/to/scorpio.toml or place scorpio.toml in the working directory ({}).",
        cwd.display()
    );
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
