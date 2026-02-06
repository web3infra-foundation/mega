use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

use serde::{Deserialize, Serialize};

// Configuration error type (using simple String for error messages)
pub type ConfigError = String;

// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Main configuration structure
#[derive(Debug, Deserialize, Serialize)]
pub struct ScorpioConfig {
    config: HashMap<String, String>,
}

const DEFAULT_LOAD_DIR_DEPTH: usize = 3;
const DEFAULT_FETCH_FILE_THREAD: usize = 10;
const DEFAULT_ANTARES_SUBDIR: &str = "antares";
const DEFAULT_DICFUSE_IMPORT_CONCURRENCY: usize = 4;

// Dicfuse timeout/cache defaults are tuned for interactive usage.
// Antares uses a separate set of build-oriented defaults below.
//
// TODO(perf):
// - Re-tune TTL/timeout defaults from production lookup metrics.
// - Split timeout knobs by request class (tree listing vs blob fetch).
// - Support per-mount overrides instead of process-global defaults.

/// Directory refresh TTL for base Dicfuse mounts.
const DEFAULT_DICFUSE_DIR_SYNC_TTL_SECS: u64 = 5;

/// Kernel entry TTL for base Dicfuse mounts.
const DEFAULT_DICFUSE_REPLY_TTL_SECS: u64 = 2;

/// Per-request timeout for directory listing RPCs.
const DEFAULT_DICFUSE_FETCH_DIR_TIMEOUT_SECS: u64 = 10;

/// TCP connect timeout for Dicfuse HTTP requests.
const DEFAULT_DICFUSE_CONNECT_TIMEOUT_SECS: u64 = 3;

/// Retry count for transient directory listing failures.
const DEFAULT_DICFUSE_FETCH_DIR_MAX_RETRIES: u32 = 3;

const DEFAULT_DICFUSE_OPEN_BUFF_MAX_BYTES: u64 = 256 * 1024 * 1024; // 256MiB
const DEFAULT_DICFUSE_OPEN_BUFF_MAX_FILES: usize = 4096;

// Antares mounts are primarily used by build workloads.

/// Preload directory depth for Antares mounts.
const DEFAULT_ANTARES_LOAD_DIR_DEPTH: usize = 3;

/// Directory refresh TTL for Antares mounts.
const DEFAULT_ANTARES_DICFUSE_DIR_SYNC_TTL_SECS: u64 = 120;

/// Kernel entry TTL for Antares mounts.
const DEFAULT_ANTARES_DICFUSE_REPLY_TTL_SECS: u64 = 60;

const DEFAULT_ANTARES_DICFUSE_OPEN_BUFF_MAX_BYTES: u64 = 64 * 1024 * 1024; // 64MiB
const DEFAULT_ANTARES_DICFUSE_OPEN_BUFF_MAX_FILES: usize = 1024;

// Global configuration management
static SCORPIO_CONFIG: OnceLock<ScorpioConfig> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DicfuseStatMode {
    Fast,
    Accurate,
}

fn parse_stat_mode(v: Option<&String>, default: DicfuseStatMode) -> DicfuseStatMode {
    match v.map(|s| s.trim().to_ascii_lowercase()) {
        Some(s) if s == "fast" => DicfuseStatMode::Fast,
        Some(s) if s == "accurate" => DicfuseStatMode::Accurate,
        Some(_) => default,
        None => default,
    }
}

/// Initialize global configuration
///
/// # Arguments
/// * `path` - Path to the configuration file
///
/// # Returns
/// `ConfigResult<()>` - Success or error
pub fn init_config(path: &str) -> ConfigResult<()> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Config file not found at '{path}': {e}"))?;

    let mut config: HashMap<String, String> =
        toml::from_str(&content).map_err(|e| format!("Invalid config format: {e}"))?;

    // Set default values and validate configuration
    set_defaults(&mut config, path)?;
    validate(&mut config)?;

    let scorpio_config = ScorpioConfig { config };
    SCORPIO_CONFIG
        .set(scorpio_config)
        .map_err(|_| "Configuration already initialized".into())
}

/// Set default values
///
/// # Arguments
/// * `config` - Mutable reference to configuration HashMap
/// * `path` - Path to save the configuration file if defaults are set
///
/// # Returns
/// `ConfigResult<()>` - Success or error
fn set_defaults(config: &mut HashMap<String, String>, path: &str) -> ConfigResult<()> {
    let username = whoami::username();
    // Prefer a writable, container-friendly default. Users can still override via scorpio.toml.
    let base_path = format!("/tmp/megadir-{username}");

    // Check if critical fields are empty (first run scenario)
    let is_first_run = config
        .get("workspace")
        .map(|s| s.is_empty())
        .unwrap_or(true)
        || config
            .get("store_path")
            .map(|s| s.is_empty())
            .unwrap_or(true);

    if is_first_run {
        // Handle workspace path
        let workspace_path = {
            let entry = config.entry("workspace".into());
            entry
                .and_modify(|v| {
                    if v.is_empty() {
                        *v = format!("{base_path}/mount")
                    }
                })
                .or_insert_with(|| format!("{base_path}/mount"))
                .to_owned()
        };

        // Handle store path
        let store_path = {
            let entry = config.entry("store_path".into());
            entry
                .and_modify(|v| {
                    if v.is_empty() {
                        *v = format!("{base_path}/store")
                    }
                })
                .or_insert_with(|| format!("{base_path}/store"))
                .to_owned()
        };

        // Antares defaults under base_path/antares
        let antares_root = format!("{base_path}/{DEFAULT_ANTARES_SUBDIR}");

        // Helper closure to ensure config entry has a non-empty value
        let mut ensure_config_with_default = |key: &str, default: String| {
            config
                .entry(key.to_string())
                .and_modify(|v| {
                    if v.is_empty() {
                        *v = default.clone();
                    }
                })
                .or_insert_with(|| default)
                .clone()
        };

        let antares_upper =
            ensure_config_with_default("antares_upper_root", format!("{antares_root}/upper"));
        let antares_cl =
            ensure_config_with_default("antares_cl_root", format!("{antares_root}/cl"));
        let antares_mount =
            ensure_config_with_default("antares_mount_root", format!("{antares_root}/mnt"));
        let antares_state =
            ensure_config_with_default("antares_state_file", format!("{antares_root}/state.toml"));
        // Create required directories
        for path in [
            workspace_path.as_str(),
            store_path.as_str(),
            antares_upper.as_str(),
            antares_cl.as_str(),
            antares_mount.as_str(),
        ] {
            let path = Path::new(path);
            if let Err(e) = fs::create_dir_all(path) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(format!(
                        "Failed to create directory {}: {}",
                        path.display(),
                        e
                    ));
                }
            }
        }

        // Ensure parent of state file exists
        if let Some(parent) = Path::new(&antares_state).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(format!(
                        "Failed to create parent directory {}: {}",
                        parent.display(),
                        e
                    ));
                }
            }
        }

        // Save updated configuration
        let toml =
            toml::to_string(&config).map_err(|e| format!("Failed to serialize config: {e}"))?;
        fs::write(path, &toml)
            .map_err(|e| format!("Failed to save config {}: {e}", Path::new(path).display()))?;
    }

    // Always ensure runtime paths exist (even when the config was fully specified).
    // This keeps `scorpio` runnable out-of-the-box when `config_file` is missing, and avoids
    // surprising panics due to missing directories under /tmp.
    for key in [
        "workspace",
        "store_path",
        "antares_upper_root",
        "antares_cl_root",
        "antares_mount_root",
    ] {
        if let Some(p) = config.get(key) {
            if !p.is_empty() {
                if let Err(e) = fs::create_dir_all(Path::new(p)) {
                    if e.kind() != std::io::ErrorKind::AlreadyExists {
                        return Err(format!("Failed to create directory {}: {}", p, e));
                    }
                }
            }
        }
    }

    if let Some(state_file) = config.get("antares_state_file") {
        if let Some(parent) = Path::new(state_file).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(format!(
                        "Failed to create parent directory {}: {}",
                        parent.display(),
                        e
                    ));
                }
            }
        }
    }

    // Ensure the `config_file` exists (default: config.toml with works=[]).
    let config_file = config
        .get("config_file")
        .ok_or("Missing 'config_file' in configuration".to_string())?;
    if !Path::new(config_file).exists() {
        fs::write(config_file, "works=[]")
            .map_err(|e| format!("Failed to create {config_file}: {e}"))?;
    }
    Ok(())
}

/// Get reference to global configuration
///
/// # Panics
/// Panics if configuration hasn't been initialized
/// Get reference to global configuration, or generate a default one if not initialized.
///
/// If the configuration hasn't been initialized, this will create a default config
/// (with sensible defaults) and initialize the global config with it.
fn get_config() -> &'static ScorpioConfig {
    SCORPIO_CONFIG.get_or_init(|| {
        // Generate sensible defaults
        let username = whoami::username();
        // Prefer a writable, container-friendly default. Users can still override via scorpio.toml.
        let base_path = format!("/tmp/megadir-{username}");
        let mut config = HashMap::new();
        config.insert("base_url".to_string(), "http://localhost:8000".to_string());
        config.insert("workspace".to_string(), format!("{base_path}/mount"));
        config.insert("store_path".to_string(), format!("{base_path}/store"));
        config.insert("git_author".to_string(), "MEGA".to_string());
        config.insert("git_email".to_string(), "admin@mega.org".to_string());
        config.insert("config_file".to_string(), "config.toml".to_string());
        config.insert(
            "load_dir_depth".to_string(),
            DEFAULT_LOAD_DIR_DEPTH.to_string(),
        );
        config.insert(
            "lfs_url".to_string(),
            "http://localhost:8000/lfs".to_string(),
        );
        config.insert("dicfuse_readable".to_string(), "true".to_string());
        config.insert(
            "fetch_file_thread".to_string(),
            DEFAULT_FETCH_FILE_THREAD.to_string(),
        );
        config.insert(
            "dicfuse_import_concurrency".to_string(),
            DEFAULT_DICFUSE_IMPORT_CONCURRENCY.to_string(),
        );
        config.insert(
            "dicfuse_dir_sync_ttl_secs".to_string(),
            DEFAULT_DICFUSE_DIR_SYNC_TTL_SECS.to_string(),
        );
        config.insert(
            "dicfuse_reply_ttl_secs".to_string(),
            DEFAULT_DICFUSE_REPLY_TTL_SECS.to_string(),
        );
        config.insert(
            "dicfuse_fetch_dir_timeout_secs".to_string(),
            DEFAULT_DICFUSE_FETCH_DIR_TIMEOUT_SECS.to_string(),
        );
        config.insert(
            "dicfuse_connect_timeout_secs".to_string(),
            DEFAULT_DICFUSE_CONNECT_TIMEOUT_SECS.to_string(),
        );
        config.insert(
            "dicfuse_fetch_dir_max_retries".to_string(),
            DEFAULT_DICFUSE_FETCH_DIR_MAX_RETRIES.to_string(),
        );
        config.insert("dicfuse_stat_mode".to_string(), "accurate".to_string());
        config.insert(
            "dicfuse_open_buff_max_bytes".to_string(),
            DEFAULT_DICFUSE_OPEN_BUFF_MAX_BYTES.to_string(),
        );
        config.insert(
            "dicfuse_open_buff_max_files".to_string(),
            DEFAULT_DICFUSE_OPEN_BUFF_MAX_FILES.to_string(),
        );

        // Antares-tuned Dicfuse knobs
        config.insert(
            "antares_load_dir_depth".to_string(),
            DEFAULT_ANTARES_LOAD_DIR_DEPTH.to_string(),
        );
        config.insert("antares_dicfuse_stat_mode".to_string(), "fast".to_string());
        config.insert(
            "antares_dicfuse_open_buff_max_bytes".to_string(),
            DEFAULT_ANTARES_DICFUSE_OPEN_BUFF_MAX_BYTES.to_string(),
        );
        config.insert(
            "antares_dicfuse_open_buff_max_files".to_string(),
            DEFAULT_ANTARES_DICFUSE_OPEN_BUFF_MAX_FILES.to_string(),
        );
        config.insert(
            "antares_dicfuse_dir_sync_ttl_secs".to_string(),
            DEFAULT_ANTARES_DICFUSE_DIR_SYNC_TTL_SECS.to_string(),
        );
        config.insert(
            "antares_dicfuse_reply_ttl_secs".to_string(),
            DEFAULT_ANTARES_DICFUSE_REPLY_TTL_SECS.to_string(),
        );
        // Antares defaults under base_path/antares
        config.insert(
            "antares_upper_root".to_string(),
            format!("{base_path}/{DEFAULT_ANTARES_SUBDIR}/upper"),
        );
        config.insert(
            "antares_cl_root".to_string(),
            format!("{base_path}/{DEFAULT_ANTARES_SUBDIR}/cl"),
        );
        config.insert(
            "antares_mount_root".to_string(),
            format!("{base_path}/{DEFAULT_ANTARES_SUBDIR}/mnt"),
        );
        config.insert(
            "antares_state_file".to_string(),
            format!("{base_path}/{DEFAULT_ANTARES_SUBDIR}/state.toml"),
        );

        // Create required directories
        for path in [config["workspace"].as_str(), config["store_path"].as_str()] {
            let path = Path::new(path);
            if let Err(e) = fs::create_dir_all(path) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    panic!("Failed to create directory {}: {}", path.display(), e);
                }
            }
        }

        // Create the config_file if it doesn't exist
        let config_file = config.get("config_file").unwrap();
        if !Path::new(config_file).exists() {
            fs::write(config_file, "works=[]")
                .unwrap_or_else(|e| panic!("Failed to create {config_file}: {e}"));
        }

        ScorpioConfig { config }
    })
}

/// Validate configuration fields
///
/// # Arguments
/// * `config` - Mutable reference to configuration HashMap
///
/// # Returns
/// `ConfigResult<()>` - Success if all required fields are present and non-empty
fn validate(config: &mut HashMap<String, String>) -> ConfigResult<()> {
    let required_keys = [
        "base_url",
        "workspace",
        "store_path",
        "git_author",
        "git_email",
        "config_file",
        "lfs_url",
        "dicfuse_readable",
        "load_dir_depth",
        "fetch_file_thread",
        "antares_upper_root",
        "antares_cl_root",
        "antares_mount_root",
        "antares_state_file",
    ];

    for key in required_keys {
        if let Some(value) = config.get(key) {
            if !value.is_empty() {
                continue;
            }
        }
        return Err(format!("Missing or empty required config: {key}"));
    }
    Ok(())
}

// Configuration accessor functions
macro_rules! config_accessor {
    ($fn_name:ident, $key:expr, $ret:ty, $default:expr) => {
        pub fn $fn_name() -> $ret {
            get_config()
                .config
                .get($key)
                .and_then(|s| s.parse().ok())
                .unwrap_or($default)
        }
    };
}

pub fn base_url() -> &'static str {
    &get_config().config["base_url"]
}

pub fn workspace() -> &'static str {
    &get_config().config["workspace"]
}

pub fn store_path() -> &'static str {
    &get_config().config["store_path"]
}

pub fn git_author() -> &'static str {
    &get_config().config["git_author"]
}

pub fn git_email() -> &'static str {
    &get_config().config["git_email"]
}

pub fn file_blob_endpoint() -> String {
    format!("{}/api/v1/file/blob", base_url())
}
pub fn tree_file_endpoint() -> String {
    format!("{}/api/v1/file/tree?path=/", base_url())
}
pub fn config_file() -> &'static str {
    &get_config().config["config_file"]
}
pub fn lfs_url() -> &'static str {
    &get_config().config["lfs_url"]
}
pub fn dicfuse_readable() -> bool {
    get_config().config["dicfuse_readable"] == "true"
}

pub fn antares_upper_root() -> &'static str {
    &get_config().config["antares_upper_root"]
}

pub fn antares_cl_root() -> &'static str {
    &get_config().config["antares_cl_root"]
}

pub fn antares_mount_root() -> &'static str {
    &get_config().config["antares_mount_root"]
}

pub fn antares_state_file() -> &'static str {
    &get_config().config["antares_state_file"]
}

config_accessor!(load_dir_depth, "load_dir_depth", usize, DEFAULT_LOAD_DIR_DEPTH);

config_accessor!(fetch_file_thread, "fetch_file_thread", usize, DEFAULT_FETCH_FILE_THREAD);

config_accessor!(dicfuse_import_concurrency, "dicfuse_import_concurrency", usize, DEFAULT_DICFUSE_IMPORT_CONCURRENCY);

config_accessor!(dicfuse_dir_sync_ttl_secs, "dicfuse_dir_sync_ttl_secs", u64, DEFAULT_DICFUSE_DIR_SYNC_TTL_SECS);

config_accessor!(dicfuse_reply_ttl_secs, "dicfuse_reply_ttl_secs", u64, DEFAULT_DICFUSE_REPLY_TTL_SECS);

config_accessor!(dicfuse_fetch_dir_timeout_secs, "dicfuse_fetch_dir_timeout_secs", u64, DEFAULT_DICFUSE_FETCH_DIR_TIMEOUT_SECS);

config_accessor!(dicfuse_connect_timeout_secs, "dicfuse_connect_timeout_secs", u64, DEFAULT_DICFUSE_CONNECT_TIMEOUT_SECS);

config_accessor!(dicfuse_fetch_dir_max_retries, "dicfuse_fetch_dir_max_retries", u32, DEFAULT_DICFUSE_FETCH_DIR_MAX_RETRIES);

pub fn dicfuse_stat_mode() -> DicfuseStatMode {
    parse_stat_mode(
        get_config().config.get("dicfuse_stat_mode"),
        DicfuseStatMode::Accurate,
    )
}

config_accessor!(dicfuse_open_buff_max_bytes, "dicfuse_open_buff_max_bytes", u64, DEFAULT_DICFUSE_OPEN_BUFF_MAX_BYTES);

config_accessor!(dicfuse_open_buff_max_files, "dicfuse_open_buff_max_files", usize, DEFAULT_DICFUSE_OPEN_BUFF_MAX_FILES);

config_accessor!(antares_load_dir_depth, "antares_load_dir_depth", usize, DEFAULT_ANTARES_LOAD_DIR_DEPTH);

config_accessor!(antares_dicfuse_dir_sync_ttl_secs, "antares_dicfuse_dir_sync_ttl_secs", u64, DEFAULT_ANTARES_DICFUSE_DIR_SYNC_TTL_SECS);

config_accessor!(antares_dicfuse_reply_ttl_secs, "antares_dicfuse_reply_ttl_secs", u64, DEFAULT_ANTARES_DICFUSE_REPLY_TTL_SECS);

pub fn antares_dicfuse_stat_mode() -> DicfuseStatMode {
    parse_stat_mode(
        get_config().config.get("antares_dicfuse_stat_mode"),
        DicfuseStatMode::Fast,
    )
}

config_accessor!(antares_dicfuse_open_buff_max_bytes, "antares_dicfuse_open_buff_max_bytes", u64, DEFAULT_ANTARES_DICFUSE_OPEN_BUFF_MAX_BYTES);

config_accessor!(antares_dicfuse_open_buff_max_files, "antares_dicfuse_open_buff_max_files", usize, DEFAULT_ANTARES_DICFUSE_OPEN_BUFF_MAX_FILES);
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_url() {
        let config_content = r#"
        base_url = "http://localhost:8000"
        workspace = ""
        store_path = ""
        git_author = "MEGA"
        git_email = "admin@mega.org"
        config_file = "config.toml"
        lfs_url = "http://localhost:8000/lfs"
        dicfuse_readable = "true"
        load_dir_depth = "3"
        fetch_file_thread = "10"
        antares_upper_root = ""
        antares_cl_root = ""
        antares_mount_root = ""
        antares_state_file = ""
        "#;
        let config_path = "/tmp/scorpio.toml";
        std::fs::write(config_path, config_content).expect("Failed to write test config file");
        match init_config(config_path) {
            Ok(()) => {
                assert_eq!(base_url(), "http://localhost:8000");
                assert_eq!(
                    workspace(),
                    format!("/tmp/megadir-{}/mount", whoami::username())
                );
                assert_eq!(
                    store_path(),
                    format!("/tmp/megadir-{}/store", whoami::username())
                );
                assert_eq!(git_author(), "MEGA");
                assert_eq!(git_email(), "admin@mega.org");
                assert_eq!(
                    file_blob_endpoint(),
                    "http://localhost:8000/api/v1/file/blob"
                );
                assert_eq!(load_dir_depth(), 3);
                assert_eq!(fetch_file_thread(), 10);
                assert_eq!(config_file(), "config.toml");
                assert!(antares_upper_root().ends_with("/antares/upper"));
                assert!(antares_cl_root().ends_with("/antares/cl"));
                assert!(antares_mount_root().ends_with("/antares/mnt"));
                assert!(antares_state_file().ends_with("/antares/state.toml"));
            }
            Err(e) if e.contains("already initialized") => {
                // Other tests may have initialized the global config first; assert basic invariants.
                assert!(!base_url().is_empty());
                assert!(!workspace().is_empty());
                assert!(!store_path().is_empty());
                assert!(file_blob_endpoint().starts_with(base_url()));
            }
            Err(e) => panic!("Failed to load config: {e}"),
        }
    }
}
