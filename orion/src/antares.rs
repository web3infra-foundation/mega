//! Antares filesystem management via scorpiofs direct calls.
//!
//! This module provides a singleton wrapper around `scorpiofs::AntaresManager`
//! for managing overlay filesystem mounts used during build operations.

use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use scorpiofs::{AntaresConfig, AntaresManager, AntaresPaths};
use tokio::sync::OnceCell;

static MANAGER: OnceCell<Arc<AntaresManager>> = OnceCell::const_new();

type DynError = Box<dyn Error + Send + Sync>;

/// Default configuration file path.
const DEFAULT_CONFIG_PATH: &str = "/etc/scorpio/scorpio.toml";

/// Get the global AntaresManager instance.
///
/// Initializes the manager on first call by loading the scorpio configuration
/// from the path specified by `SCORPIO_CONFIG` environment variable, or
/// `/etc/scorpio/scorpio.toml` if not set.
async fn get_manager() -> Result<&'static Arc<AntaresManager>, DynError> {
    MANAGER
        .get_or_try_init(|| async {
            let config_path = resolve_or_generate_config_path()?;
            let config_path_str = config_path
                .to_str()
                .ok_or_else(|| io_other("Invalid SCORPIO_CONFIG path (non-UTF8)") as DynError)?;

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
    io::Error::new(io::ErrorKind::Other, message.into())
}

fn resolve_or_generate_config_path() -> Result<PathBuf, DynError> {
    if let Ok(path) = std::env::var("SCORPIO_CONFIG") {
        let config_path = PathBuf::from(path);
        if config_path.exists() {
            return Ok(config_path);
        }
        return Err(io_other(format!(
            "SCORPIO_CONFIG is set but file does not exist: {}",
            config_path.display()
        ))
        .into());
    }

    let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
    if default_path.exists() {
        return Ok(default_path);
    }

    // Fall back to generating a minimal config under BUILD_TMP so the worker can
    // run even when /etc is not provisioned (e.g. ad-hoc local runs or CI smoke).
    generate_minimal_config()
}

fn generate_minimal_config() -> Result<PathBuf, DynError> {
    let build_tmp = std::env::var("BUILD_TMP").unwrap_or_else(|_| "/tmp/orion-builds".to_string());
    let runtime_root = PathBuf::from(build_tmp).join("scorpio-runtime");

    let store_path = runtime_root.join("store");
    let antares_root = runtime_root.join("antares");
    let antares_upper_root = antares_root.join("upper");
    let antares_cl_root = antares_root.join("cl");
    let antares_mount_root = antares_root.join("mnt");
    let antares_state_file = antares_root.join("state.toml");

    for dir in [
        &store_path,
        &antares_upper_root,
        &antares_cl_root,
        &antares_mount_root,
    ] {
        fs::create_dir_all(dir)?;
    }

    let base_url =
        std::env::var("MEGA_BASE_URL").unwrap_or_else(|_| "http://git.gitmega.com".to_string());
    let lfs_url = std::env::var("MEGA_LFS_URL").unwrap_or_else(|_| base_url.clone());

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let config_path = runtime_root.join(format!("scorpio.{stamp}.toml"));

    let contents = format!(
        "base_url = \"{base_url}\"\n\
lfs_url = \"{lfs_url}\"\n\
store_path = \"{}\"\n\
antares_upper_root = \"{}\"\n\
antares_cl_root = \"{}\"\n\
antares_mount_root = \"{}\"\n\
antares_state_file = \"{}\"\n\
antares_load_dir_depth = \"3\"\n\
antares_dicfuse_stat_mode = \"fast\"\n\
antares_dicfuse_open_buff_max_bytes = \"67108864\"\n\
antares_dicfuse_open_buff_max_files = \"1024\"\n\
antares_dicfuse_dir_sync_ttl_secs = \"120\"\n\
antares_dicfuse_reply_ttl_secs = \"60\"\n",
        store_path.display(),
        antares_upper_root.display(),
        antares_cl_root.display(),
        antares_mount_root.display(),
        antares_state_file.display(),
    );

    write_file_atomic(&config_path, contents.as_bytes())?;

    tracing::warn!(
        "Scorpio config not found; generated a minimal config at {}. \
Set SCORPIO_CONFIG to a persistent path for production deployments.",
        config_path.display()
    );

    Ok(config_path)
}

fn write_file_atomic(path: &Path, contents: &[u8]) -> Result<(), DynError> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, contents)?;
    fs::rename(tmp_path, path)?;
    Ok(())
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
