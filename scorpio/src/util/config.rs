use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

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

// Global configuration management
static SCORPIO_CONFIG: OnceLock<ScorpioConfig> = OnceLock::new();

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
    let base_path = format!("/home/{username}/megadir");

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

        // Create the config.toml
        let config_file = config
            .get("config_file")
            .ok_or("Missing 'config_file' in configuration".to_string())?;
        if !Path::new(config_file).exists() {
            fs::write(config_file, "works=[]").map_err(|e| {
                format!(
                    "Failed to create {}: {}",
                    config.get("config_file").unwrap(),
                    e
                )
            })?;
        }
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
        let base_path = format!("/home/{username}/megadir");
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

///Get the depth of directory loading
pub fn load_dir_depth() -> usize {
    get_config()
        .config
        .get("load_dir_depth")
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LOAD_DIR_DEPTH)
}

///Get the number of file download threads
pub fn fetch_file_thread() -> usize {
    get_config()
        .config
        .get("fetch_file_thread")
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_FETCH_FILE_THREAD)
}
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
        let _ = init_config(config_path);
        assert_eq!(base_url(), "http://localhost:8000");
        assert_eq!(
            workspace(),
            format!("/home/{}/megadir/mount", whoami::username())
        );
        assert_eq!(
            store_path(),
            format!("/home/{}/megadir/store", whoami::username())
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
        assert!(antares_upper_root().ends_with("/megadir/antares/upper"));
        assert!(antares_cl_root().ends_with("/megadir/antares/cl"));
        assert!(antares_mount_root().ends_with("/megadir/antares/mnt"));
        assert!(antares_state_file().ends_with("/megadir/antares/state.toml"));
    }
}
