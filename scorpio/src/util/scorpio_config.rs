extern crate serde;
extern crate toml;

use std::collections::HashMap;
use std::fs;
use serde::Deserialize;
use std::sync::OnceLock;

/// Represents the configuration structure parsed from a TOML file.
/// Uses a HashMap to store key-value pairs of configuration settings.
#[derive(Debug, Deserialize)]
pub struct ScorpioConfig {
    config: HashMap<String, String>,
}

impl ScorpioConfig {
    /// Loads configuration from a TOML file at the given path.
    /// Panics if the file cannot be read or parsed.
    fn from_file(path: &str) -> Self {
        let content = fs::read_to_string(path)
            .expect("Failed to read the configuration file.");
        toml::from_str(&content)
            .expect("Failed to parse the configuration file.")
    }

    /// Retrieves the value associated with the given configuration key.
    /// Returns `Some(&str)` if the key exists, otherwise `None`.
    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.config.get(key).map(|s| s.as_str())
    }
}

/// Global static instance of the configuration, initialized only once.
static SCORPIO_CONFIG: OnceLock<ScorpioConfig> = OnceLock::new();

/// Returns a reference to the global configuration instance.
pub fn get_config() -> &'static ScorpioConfig {
    SCORPIO_CONFIG.get_or_init(|| ScorpioConfig::from_file("scorpio.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_url() {
        let config = get_config();
        assert_eq!(config.get_value("file_blob_endpoint"), Some("http://localhost:8000/api/v1/file/blob"));
    }

}