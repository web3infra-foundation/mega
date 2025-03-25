use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
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

impl ScorpioConfig {
    /// Load configuration from a file
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    /// `ConfigResult<Self>` - Loaded configuration or error
    pub fn from_file(path: &str) -> ConfigResult<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Config file not found at '{}': {}", path, e))?;

        let mut config: HashMap<String, String> = toml::from_str(&content)
            .map_err(|e| format!("Invalid config format: {}", e))?;

        // Set default values and validate configuration
        Self::set_defaults(&mut config,path)?;
        Self::validate(&mut config)?;

        Ok(Self { config })
    }

    /// Set default values
    ///
    /// # Arguments
    /// * `config` - Mutable reference to configuration HashMap
    /// * `path` - Path to save the configuration file if defaults are set
    ///
    /// # Returns
    /// `ConfigResult<()>` - Success or error
    fn set_defaults(config: &mut HashMap<String, String>,path:&str) -> ConfigResult<()> {
        let username = whoami::username();
        let base_path = format!("/home/{}/megadir", username);

        // Check if critical fields are empty (first run scenario)
        let is_first_run = config.get("workspace").map(|s| s.is_empty()).unwrap_or(true)
            || config.get("store_path").map(|s| s.is_empty()).unwrap_or(true);

        if is_first_run {
            // Handle workspace path
            let workspace_path = {
                let entry = config.entry("workspace".into());
                entry.and_modify(|v| if v.is_empty() { *v = format!("{}/mount", base_path) })
                    .or_insert_with(|| format!("{}/mount", base_path))
                    .to_owned() 
            };

            // Handle store path
            let store_path = {
                let entry = config.entry("store_path".into());
                entry.and_modify(|v| if v.is_empty() { *v = format!("{}/store", base_path) })
                    .or_insert_with(|| format!("{}/store", base_path))
                    .to_owned()
            };
            
            // Create required directories
            for path in [workspace_path.as_str(), store_path.as_str()] {
                let path = Path::new(path);
                if let Err(e) = fs::create_dir_all(path) {
                    if e.kind() != std::io::ErrorKind::AlreadyExists {
                        return Err(format!("Failed to create directory {}: {}", path.display(), e));
                    }
                }
            }

            // Save updated configuration
            let toml = toml::to_string(&config)
                .expect("Failed to serialize config");
            fs::write(path, toml)
                .unwrap_or_else(|e| panic!("Failed to save config: {}", e));
        }
        Ok(())
    }

    // The following methods can be safely called as validation is done during loading
    pub fn base_url(&self) -> &str { &self.config["base_url"] }

    pub fn workspace(&self) -> &str { &self.config["workspace"] }

    pub fn store_path(&self) -> &str { &self.config["store_path"] }

    pub fn git_author(&self) -> &str { &self.config["git_author"] }

    pub fn git_email(&self) -> &str { &self.config["git_email"] }

    pub fn file_blob_endpoint(&self) -> &str { &self.config["file_blob_endpoint"] }

    pub fn config_file(&self) -> &str { &self.config["config_file"] }

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
            "file_blob_endpoint",
            "config_file",
        ];

        for key in required_keys {
            if let Some(value) = config.get(key) {
                if !value.is_empty() {
                    continue;
                }
            }
            return Err(format!("Missing or empty required config: {}", key));
        }
        Ok(())
    }
}

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
    let config = ScorpioConfig::from_file(path)?;
    SCORPIO_CONFIG.set(config)
        .map_err(|_| "Configuration already initialized".into())
}

/// Get reference to global configuration
///
/// # Panics
/// Panics if configuration hasn't been initialized
pub fn get_config() -> &'static ScorpioConfig {
    SCORPIO_CONFIG.get().expect("Configuration not initialized")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_url() {
        init_config("scorpio.toml").unwrap();
        let config = get_config();
        assert_eq!(config.base_url(), "http://localhost:8000");
        assert_eq!(config.workspace(), format!("/home/{}/megadir/mount", whoami::username()));
        assert_eq!(config.store_path(), format!("/home/{}/megadir/store", whoami::username()));
        assert_eq!(config.git_author(), "MEGA");
        assert_eq!(config.git_email(), "admin@mega.org");
        assert_eq!(config.file_blob_endpoint(), "http://localhost:8000/api/v1/file/blob");
        assert_eq!(config.config_file(), "config.toml");
    }

}