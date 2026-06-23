use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::sync::RwLock;

/// Target environment configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TargetConfig {
    /// Orion WebSocket server URL
    pub server_ws: String,
    /// Scorpio base URL (replaces base_url in scorpio.toml)
    pub scorpio_base_url: String,
    /// Scorpio LFS URL (replaces lfs_url in scorpio.toml)
    pub scorpio_lfs_url: String,
}

/// Target configuration store loaded from JSON file
#[derive(Debug, Clone)]
pub struct Config {
    /// Map from target name (e.g., "aws-gitmega") to its configuration
    targets: HashMap<String, TargetConfig>,
    /// Directory to save Orion logs
    log_dir: String,
    /// Path to the Orion source directory (runner-config, systemd, etc.)
    orion_source_dir: String,
    /// Path to the Orion binary to deploy
    orion_binary_path: String,
    /// Path to the SSH public key for VM access
    ssh_public_key_path: String,
}

/// Expand a leading `~` or `~/` to `$HOME`. Other paths are returned unchanged.
pub fn expand_tilde(path: impl AsRef<str>) -> PathBuf {
    let path = path.as_ref();
    if path == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(path));
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

impl Config {
    /// Create a new Config with the given log directory and empty targets
    #[cfg(test)]
    pub fn new(
        log_dir: String,
        orion_source_dir: String,
        orion_binary_path: String,
        ssh_public_key_path: String,
        targets: HashMap<String, TargetConfig>,
    ) -> Self {
        Self {
            targets,
            log_dir,
            orion_source_dir,
            orion_binary_path,
            ssh_public_key_path,
        }
    }

    /// Load configuration from a JSON file.
    ///
    /// Errors are annotated with the absolute path that was attempted, so
    /// callers (and users staring at the log) can tell exactly which file
    /// the loader was looking for.
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let abs = absolutize(path);
        let content = tokio::fs::read_to_string(path).await.with_context(|| {
            format!(
                "failed to read config file at {} \
                 (set CONFIG_PATH or run from a directory containing target_config.json)",
                abs.display()
            )
        })?;
        let parsed: ConfigFile = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse JSON config at {}", abs.display()))?;

        let orion_source_dir = parsed.orion_source_dir.ok_or_else(|| {
            anyhow::anyhow!(
                "missing required field 'orion_source_dir' in config file {}",
                abs.display()
            )
        })?;
        let orion_binary_path = parsed.orion_binary_path.ok_or_else(|| {
            anyhow::anyhow!(
                "missing required field 'orion_binary_path' in config file {}",
                abs.display()
            )
        })?;
        let ssh_public_key_path = parsed.ssh_public_key_path.ok_or_else(|| {
            anyhow::anyhow!(
                "missing required field 'ssh_public_key_path' in config file {}",
                abs.display()
            )
        })?;

        let mut targets = HashMap::new();
        for (name, config) in parsed.targets {
            targets.insert(name, config);
        }

        Ok(Config {
            targets,
            log_dir: parsed
                .log_dir
                .unwrap_or_else(|| "/var/log/orion-scheduler".to_string()),
            orion_source_dir,
            orion_binary_path,
            ssh_public_key_path,
        })
    }

    /// Get configuration for a specific target
    pub fn get(&self, target: &str) -> Option<&TargetConfig> {
        self.targets.get(target)
    }

    /// Get all available target names
    pub fn target_names(&self) -> Vec<&String> {
        self.targets.keys().collect()
    }

    /// Get the log directory path
    pub fn log_dir(&self) -> &str {
        &self.log_dir
    }

    /// Get the Orion source directory path
    pub fn orion_source_dir(&self) -> &str {
        &self.orion_source_dir
    }

    /// Get the Orion binary path
    pub fn orion_binary_path(&self) -> &str {
        &self.orion_binary_path
    }

    /// Get the SSH public key path for VM access
    pub fn ssh_public_key_path(&self) -> &str {
        &self.ssh_public_key_path
    }
}

/// Locate `target_config.json` automatically when the operator has not set
/// `CONFIG_PATH`.
///
/// Candidates are tried in order and the first existing path wins:
/// 1. `./target_config.json` — preserves the historical behaviour of
///    `cargo run` inside `orion-scheduler/`.
/// 2. `<exe_dir>/target_config.json` — convenient for shipped binaries that
///    co-locate the config next to the executable.
/// 3. `<crate root>/target_config.json` — `CARGO_MANIFEST_DIR` is baked in
///    at compile time, so `cargo run --bin orion-scheduler` from the mega
///    workspace root still picks up the crate-local config.
///
/// Returns `None` when none of the candidates exist; callers should report
/// the full search list so users know where to drop the file or which env
/// var to set.
pub fn default_config_path() -> Option<PathBuf> {
    default_config_candidates()
        .into_iter()
        .find(|p| p.is_file())
}

/// Return the ordered list of paths inspected by [`default_config_path`].
/// Exposed so the caller can log them on failure.
pub fn default_config_candidates() -> Vec<PathBuf> {
    const FILE_NAME: &str = "target_config.json";
    let mut out: Vec<PathBuf> = Vec::new();

    out.push(PathBuf::from(FILE_NAME));

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.join(FILE_NAME));
    }

    out.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FILE_NAME));

    out
}

/// Canonicalize when possible so error messages always print an absolute
/// path. Falls back to a best-effort cwd join when the file does not yet
/// exist (`canonicalize` requires the path to exist).
fn absolutize(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path.to_path_buf(),
    }
}

/// Internal structure for parsing the JSON config file
#[derive(Debug, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    targets: HashMap<String, TargetConfig>,
    #[serde(default)]
    log_dir: Option<String>,
    #[serde(default)]
    orion_source_dir: Option<String>,
    #[serde(default)]
    orion_binary_path: Option<String>,
    #[serde(default)]
    ssh_public_key_path: Option<String>,
}

/// Global configuration state
pub type SharedConfig = Arc<RwLock<Config>>;

#[cfg(test)]
mod tests {
    use super::expand_tilde;

    #[test]
    fn expand_tilde_home_prefix() {
        let home = std::env::var("HOME").expect("HOME must be set in test env");
        let p = expand_tilde("~/.local/share/qlean/images/debian-13-buck2.qcow2");
        assert_eq!(
            p,
            std::path::PathBuf::from(home).join(".local/share/qlean/images/debian-13-buck2.qcow2")
        );
    }

    #[test]
    fn expand_tilde_absolute_unchanged() {
        let abs = "/home/orion/image.qcow2";
        assert_eq!(expand_tilde(abs), std::path::PathBuf::from(abs));
    }
}
