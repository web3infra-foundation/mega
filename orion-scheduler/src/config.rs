use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
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

    /// Load configuration from a JSON file
    pub async fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let parsed: ConfigFile = serde_json::from_str(&content)?;

        let orion_source_dir = parsed.orion_source_dir.ok_or_else(|| {
            anyhow::anyhow!("missing required field 'orion_source_dir' in config file")
        })?;
        let orion_binary_path = parsed.orion_binary_path.ok_or_else(|| {
            anyhow::anyhow!("missing required field 'orion_binary_path' in config file")
        })?;
        let ssh_public_key_path = parsed.ssh_public_key_path.ok_or_else(|| {
            anyhow::anyhow!("missing required field 'ssh_public_key_path' in config file")
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
