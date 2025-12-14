//! Configuration management for the Mono and Mega application
//! This module provides functionality to load, parse, and manage configuration settings

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use c::{ConfigError, FileFormat};
pub use config as c;

use config::builder::DefaultState;
use config::{Source, ValueKind};
use serde::{Deserialize, Deserializer, Serialize};

use callisto::sea_orm_active_enums::StorageTypeEnum;

use crate::utils;

/// Retrieves the base directory path for Mega
///
/// The directory is determined in the following priority order:
/// 1. Uses the `MEGA_BASE_DIR` environment variable if set
/// 2. Falls back to system default paths when environment variable is not set:
///     - On Linux: `~/.local/share/mega`
///     - On Windows: `C:\Users\{UserName}\AppData\Local\mega`
///     - On macOS: `~/Library/Application Support/mega`
///
/// # Returns
/// A PathBuf containing the base directory path
///
/// # Panics
/// Will panic if both conditions occur:
/// - Environment variable is not set
/// - System base directories cannot be determined
///
pub fn mega_base() -> PathBuf {
    // Get the base directory from the environment variable or use the default
    let base_dir = std::env::var("MEGA_BASE_DIR").unwrap_or_else(|_| {
        let base_dirs = directories::BaseDirs::new().unwrap();
        base_dirs
            .data_local_dir()
            .join("mega")
            .to_str()
            .unwrap()
            .to_string()
    });

    PathBuf::from(base_dir)
}

/// Retrieves the cache directory path for Mega
///
/// The directory is determined in the following priority order:
/// 1. Uses the `MEGA_CACHE_DIR` environment variable if set
/// 2. Falls back to system default paths when environment variable is not set:
///     - On Linux: `~/.cache/mega`
///     - On Windows: `C:\Users\{username}\AppData\Local\Cache\mega`
///     - On macOS: `~/Library/Caches/mega`
///
/// # Returns
/// A PathBuf containing the cache directory path
///
/// # Panics
/// Will panic if both conditions occur:
/// - Environment variable is not set
/// - System cache directories cannot be determined
///
pub fn mega_cache() -> PathBuf {
    // Get the cache directory from the environment variable or use the default
    let cache_dir = std::env::var("MEGA_CACHE_DIR").unwrap_or_else(|_| {
        let base_dirs = directories::BaseDirs::new().unwrap();
        base_dirs
            .cache_dir()
            .join("mega")
            .to_str()
            .unwrap()
            .to_string()
    });

    PathBuf::from(cache_dir)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub base_dir: PathBuf,
    pub log: LogConfig,
    pub database: DbConfig,
    pub monorepo: MonoConfig,
    pub pack: PackConfig,
    pub authentication: AuthConfig,
    pub lfs: LFSConfig,
    #[serde(default)]
    pub blame: BlameConfig,
    // Not used in mega app
    #[serde(default)]
    pub oauth: Option<OauthConfig>,
    pub build: BuildConfig,
    pub redis: RedisConfig,
    #[serde(default)]
    pub buck: Option<BuckConfig>,
}

impl Config {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let builder = c::Config::builder()
            .add_source(c::File::new(path, FileFormat::Toml))
            .add_source(
                c::Environment::with_prefix("mega")
                    .prefix_separator("_")
                    .separator("__"),
            );

        let config = variable_placeholder_substitute(builder);

        Config::from_config(config)
    }

    pub fn mock() -> Self {
        Self {
            base_dir: PathBuf::new(),
            log: LogConfig::default(),
            database: DbConfig::default(),
            monorepo: MonoConfig::default(),
            pack: PackConfig::default(),
            authentication: AuthConfig::default(),
            lfs: LFSConfig::default(),
            blame: BlameConfig::default(),
            oauth: None,
            build: BuildConfig::default(),
            redis: RedisConfig::default(),
            buck: None,
        }
    }

    pub fn load_str(content: &str) -> Result<Self, ConfigError> {
        let builder = c::Config::builder()
            .add_source(c::File::from_str(content, FileFormat::Toml))
            .add_source(
                c::Environment::with_prefix("mega")
                    .prefix_separator("_")
                    .separator("__"),
            );

        let config = variable_placeholder_substitute(builder);

        Config::from_config(config)
    }

    pub fn load_sources<T>(sources: Vec<Box<T>>) -> Result<Self, ConfigError>
    where
        T: Source + Send + Sync + 'static,
    {
        let mut builder = c::Config::builder();
        for source in sources {
            builder = builder.add_source(*source);
        }

        let config = variable_placeholder_substitute(builder);

        Config::from_config(config)
    }

    pub fn from_config(config: c::Config) -> Result<Self, ConfigError> {
        config.try_deserialize::<Config>()
    }

    pub fn enable_http_auth(&self) -> bool {
        self.authentication.enable_http_auth
    }
}

impl Default for Config {
    fn default() -> Self {
        let base_dir = mega_base();
        std::fs::create_dir_all(&base_dir).unwrap();

        let bin_name = utils::get_current_bin_name();
        let default_config = match bin_name.as_str() {
            "mono" => include_str!("../../config/config.toml"),
            &_ => todo!(),
        };
        let default_config = default_config
            .lines()
            .map(|line| {
                if line.starts_with("base_dir ") {
                    format!("base_dir = {base_dir:?}")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        // default config path: $MEGA_BASE_DIR/etc/config.toml
        // ensure the directory exists
        std::fs::create_dir_all(base_dir.join("etc")).unwrap();
        let config_path = base_dir.join("etc").join("config.toml");
        std::fs::write(&config_path, default_config).unwrap();
        eprintln!("create default config.toml in {:?}", &config_path);

        Config::new(config_path.to_str().unwrap()).unwrap()
    }
}

/// supports braces-delimited variables (i.e. ${foo}) in config.
/// ### Example:
/// ```toml
/// base_dir = "/tmp/.mega"
/// [log]
/// log_path = "${base_dir}/logs"
/// ```
/// ### Limitations:
/// - only support `String` type.
/// - vars apply from up to down
fn variable_placeholder_substitute(mut builder: c::ConfigBuilder<DefaultState>) -> c::Config {
    // `Config::set` is deprecated, use `ConfigBuilder::set_override` instead
    let config = builder.clone().build().unwrap(); // initial config
    let mut vars = HashMap::new();
    // top-level variables
    for (k, mut v) in config.collect().unwrap() {
        // a copy
        if let ValueKind::String(str) = &v.kind {
            if envsubst::is_templated(str) {
                let new_str = envsubst::substitute(str, &vars).unwrap();
                v.kind = ValueKind::String(new_str.clone());
                builder = builder.set_override(&k, v).unwrap();
                vars.insert(k, new_str);
            } else {
                vars.insert(k, str.clone());
            }
        }
    }
    // second-level or nested variables
    // extract all config k-v
    let map = Rc::new(RefCell::new(HashMap::new()));
    for (k, v) in config.collect().unwrap() {
        if let ValueKind::Table(_) = v.kind {
            let map_c = map.clone();
            traverse_config(&k, &v, &move |key: &str, value: &c::Value| {
                if let ValueKind::String(_) = value.kind {
                    map_c.borrow_mut().insert(key.to_string(), value.clone());
                }
            });
        }
    }

    // do substitution: ${} -> real value
    for (k, mut v) in Rc::try_unwrap(map).unwrap().into_inner() {
        let mut str = v.clone().into_string().unwrap();
        if envsubst::is_templated(&str) {
            let new_str = envsubst::substitute(&str, &vars).unwrap();
            // println!("{}: {} -> {}", k, str, &new_str);
            v.kind = ValueKind::String(new_str.clone());
            builder = builder.set_override(&k, v).unwrap();
            str = new_str;
        }
        vars.insert(k, str);
    }

    builder.build().unwrap()
}

/// visitor pattern: traverse each config & execute the closure `f`
fn traverse_config(key: &str, value: &c::Value, f: &impl Fn(&str, &c::Value)) {
    match &value.kind {
        ValueKind::Table(table) => {
            for (k, v) in table.iter() {
                // join keys by '.'
                let new_key = if key.is_empty() {
                    k.clone()
                } else {
                    format!("{key}.{k}")
                };
                traverse_config(&new_key, v, f);
            }
        }
        _ => f(key, value),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogConfig {
    pub log_path: PathBuf,
    pub level: String,
    pub print_std: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_path: mega_cache().join("logs"),
            level: String::from("info"),
            print_std: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbConfig {
    pub db_type: String,
    pub db_path: PathBuf,
    pub db_url: String,
    pub max_connection: u32,
    pub min_connection: u32,
    pub acquire_timeout: u64,
    pub connect_timeout: u64,
    pub sqlx_logging: bool,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            db_type: String::from("sqlite"),
            db_path: mega_base().join("mega.db"),
            db_url: String::from("postgres://mega:mega@localhost:5432/mega"),
            max_connection: 16,
            min_connection: 8,
            acquire_timeout: 5,
            connect_timeout: 5,
            sqlx_logging: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MonoConfig {
    pub import_dir: PathBuf,
    pub admin: String,
    pub root_dirs: Vec<String>,
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            import_dir: PathBuf::from("/third-party"),
            admin: String::from("admin"),
            root_dirs: vec![
                "third-party".to_string(),
                "project".to_string(),
                "doc".to_string(),
                "release".to_string(),
            ],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthConfig {
    pub enable_http_auth: bool,
    pub enable_test_user: bool,
    pub test_user_name: String,
    pub test_user_token: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enable_http_auth: false,
            enable_test_user: false,
            test_user_name: String::from("mega"),
            test_user_token: String::from("mega"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackConfig {
    #[serde(deserialize_with = "string_or_usize")]
    pub pack_decode_mem_size: String,
    #[serde(deserialize_with = "string_or_usize")]
    pub pack_decode_disk_size: String,
    pub pack_decode_cache_path: PathBuf,
    pub clean_cache_after_decode: bool,
    pub channel_message_size: usize,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            pack_decode_mem_size: "4G".to_string(),
            pack_decode_disk_size: "20%".to_string(),
            pack_decode_cache_path: mega_cache().join("pack_decode_cache"),
            clean_cache_after_decode: true,
            channel_message_size: 1_000_000,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: String::from("redis://127.0.0.1:6379"),
        }
    }
}

fn string_or_usize<'deserialize, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'deserialize>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrUSize {
        String(String),
        USize(usize),
    }

    Ok(match StringOrUSize::deserialize(deserializer)? {
        StringOrUSize::String(v) => v,
        StringOrUSize::USize(v) => v.to_string(),
    })
}

impl PackConfig {
    /// Converts a size string to bytes
    /// Supports formats:
    /// - Bytes with units: "1MB", "2MiB", "3GB", "4GiB"
    /// - Percentage of total memory: "1%", "50%"
    /// - Decimal ratio of total memory: "0.01", "0.5"
    /// - For compatibility: Any integer greater than or equal to 1, for example "1" will be interpreted as 1Gib.
    ///
    /// # Examples
    /// ```
    /// use common::config::PackConfig;
    ///
    /// assert_eq!(PackConfig::get_size_from_str("1MB", || Ok(1 * 1000 * 1000)).unwrap(), 1 * 1000 * 1000);
    /// assert_eq!(PackConfig::get_size_from_str("2MiB", || Ok(2 * 1024 * 1024)).unwrap(), 2 * 1024 * 1024);
    /// assert_eq!(PackConfig::get_size_from_str("3GB", || Ok(3 * 1000 * 1000 * 1000)).unwrap(), 3 * 1000 * 1000 * 1000);
    /// assert_eq!(PackConfig::get_size_from_str("4GiB", || Ok(4 * 1024 * 1024 * 1024)).unwrap(), 4 * 1024 * 1024 * 1024);
    /// assert_eq!(PackConfig::get_size_from_str("4G", || Ok(4 * 1024 * 1024 * 1024)).unwrap(), 4 * 1024 * 1024 * 1024);
    /// assert_eq!(PackConfig::get_size_from_str("1%", || Ok(100)).unwrap(), 1);
    /// assert_eq!(PackConfig::get_size_from_str("50%", || Ok(100)).unwrap(), 50);
    /// assert_eq!(PackConfig::get_size_from_str("0.01", || Ok(100)).unwrap(), 1);
    /// assert_eq!(PackConfig::get_size_from_str("0.5", || Ok(100)).unwrap(), 50);
    /// assert_eq!(PackConfig::get_size_from_str("1", || Ok(100)).unwrap(), 1 * 1024 * 1024 * 1024);
    /// ```
    /// # Notes
    /// - fn_get_total_capacity is a function that returns the total memory capacity in bytes.
    ///   If the function fails, it returns a String error message.
    pub fn get_size_from_str(
        size_str: &str,
        fn_get_total_capacity: fn() -> Result<usize, String>,
    ) -> Result<usize, String> {
        let size_str = size_str.trim();

        // Try to parse as percentage or decimal ratio
        if size_str.ends_with('%') {
            let percentage: f64 = size_str
                .trim_end_matches('%')
                .parse()
                .map_err(|_| format!("Invalid percentage: {size_str}"))?;
            let total_mem = fn_get_total_capacity()?;

            return Ok((total_mem as f64 * percentage / 100.0) as usize);
        }

        let ratio_result = size_str.parse::<f64>();
        if let Ok(ratio) = ratio_result
            && ratio > 0.0
            && ratio < 1.0
        {
            let total_mem = fn_get_total_capacity()?;

            return Ok((total_mem as f64 * ratio) as usize);
        }

        // Parse size with units
        let mut chars = size_str.chars().peekable();
        let mut number = String::new();

        // Parse the numeric part
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                number.push(c);
                chars.next();
            } else {
                break;
            }
        }

        let value: f64 = number
            .parse()
            .map_err(|_| format!("Invalid size: {size_str}"))?;
        let unit = chars.collect::<String>().to_uppercase();

        // For compatibility,
        // old configuration files use integer and use GiB as the default unit.
        if unit.is_empty() {
            return Ok((value * 1024.0 * 1024.0 * 1024.0) as usize);
        }

        let bytes = match unit.as_str() {
            "B" => value,
            "KB" => value * 1_000.0,
            "MB" => value * 1_000.0 * 1_000.0,
            "GB" => value * 1_000.0 * 1_000.0 * 1_000.0,
            "TB" => value * 1_000.0 * 1_000.0 * 1_000.0 * 1_000.0,
            "KIB" | "K" => value * 1_024.0,
            "MIB" | "M" => value * 1_024.0 * 1_024.0,
            "GIB" | "G" => value * 1_024.0 * 1_024.0 * 1_024.0,
            "TIB" | "T" => value * 1_099_511_627_776.0,
            _ => Err(format!("Invalid unit: {unit}"))?,
        };

        Ok(bytes as usize)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LFSConfig {
    pub storage_type: StorageType,
    pub local: LFSLocalConfig,
    pub aws: LFSAwsConfig,
    pub ssh: LFSSshConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageType {
    Database,
    LocalFs,
    AwsS3,
}

impl From<StorageTypeEnum> for StorageType {
    fn from(value: StorageTypeEnum) -> Self {
        match value {
            StorageTypeEnum::Database => StorageType::Database,
            StorageTypeEnum::LocalFs => StorageType::LocalFs,
            StorageTypeEnum::AwsS3 => StorageType::AwsS3,
        }
    }
}

impl From<StorageType> for StorageTypeEnum {
    fn from(value: StorageType) -> Self {
        match value {
            StorageType::Database => StorageTypeEnum::Database,
            StorageType::LocalFs => StorageTypeEnum::LocalFs,
            StorageType::AwsS3 => StorageTypeEnum::AwsS3,
        }
    }
}

impl Default for LFSConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::LocalFs,
            local: LFSLocalConfig::default(),
            aws: LFSAwsConfig::default(),
            ssh: LFSSshConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LFSLocalConfig {
    pub lfs_file_path: PathBuf,
    pub enable_split: bool,
    #[serde(deserialize_with = "string_or_usize")]
    pub split_size: String,
}

impl Default for LFSLocalConfig {
    fn default() -> Self {
        Self {
            lfs_file_path: mega_base().join("lfs"),
            enable_split: true,
            split_size: "20M".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LFSAwsConfig {
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_access_key_id: String,
    pub s3_secret_access_key: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LFSSshConfig {
    pub http_url: String,
}

impl Default for LFSSshConfig {
    fn default() -> Self {
        Self {
            http_url: "http://localhost:8000".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OauthConfig {
    pub github_client_id: String,
    pub github_client_secret: String,
    pub ui_domain: String,
    pub cookie_domain: String,
    pub campsite_api_domain: String,
    pub allowed_cors_origins: Vec<String>,
}

impl Default for OauthConfig {
    fn default() -> Self {
        Self {
            github_client_id: String::new(),
            github_client_secret: String::new(),
            ui_domain: "http://localhost".to_string(),
            cookie_domain: "localhost".to_string(),
            campsite_api_domain: "http://api.gitmono.test:3001".to_string(),
            allowed_cors_origins: vec![
                "http://localhost",
                "http://app.gitmega.com",
                "http://app.gitmono.test",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlameConfig {
    /// Maximum number of lines before considering a file as large
    pub max_lines_threshold: usize,
    /// Maximum file size in bytes before considering a file as large
    #[serde(deserialize_with = "string_or_usize")]
    pub max_size_threshold: String,
    /// Default chunk size for streaming operations when processing large files
    pub default_chunk_size: usize,
    /// Maximum number of commits to process in memory at once during blame traversal
    pub max_commits_in_memory: usize,
    /// Enable caching of intermediate blame results for better performance
    pub enable_caching: bool,
}

impl Default for BlameConfig {
    fn default() -> Self {
        Self {
            max_lines_threshold: 5000,
            max_size_threshold: "1MB".to_string(),
            default_chunk_size: 100,
            max_commits_in_memory: 50,
            enable_caching: true,
        }
    }
}

impl BlameConfig {
    /// Converts the max_size_threshold string to bytes using the same logic as PackConfig
    pub fn get_max_size_bytes(&self) -> Result<usize, String> {
        PackConfig::get_size_from_str(&self.max_size_threshold, || {
            // Default to 8GB total memory for calculation if needed
            Ok(8 * 1024 * 1024 * 1024)
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BuildConfig {
    pub enable_build: bool,
    pub orion_server: String,
}

/// Buck upload API configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuckConfig {
    /// Session timeout in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_session_timeout")]
    pub session_timeout: u64,

    /// Maximum file size (default: "100MB")
    #[serde(default = "default_max_file_size")]
    pub max_file_size: String,

    /// Maximum number of files per session (default: 1000)
    #[serde(default = "default_max_files")]
    pub max_files: u32,

    /// Maximum concurrent uploads (default: 5) - returned to client as suggestion
    #[serde(default = "default_max_concurrent_uploads")]
    pub max_concurrent_uploads: u32,

    /// Global upload concurrency limit (default: 50)
    /// Controls the total number of concurrent upload requests
    #[serde(default = "default_upload_concurrency_limit")]
    pub upload_concurrency_limit: u32,

    /// Large file concurrency limit (default: 10)
    /// Controls the number of concurrent large file uploads to prevent memory exhaustion
    #[serde(default = "default_large_file_concurrency_limit")]
    pub large_file_concurrency_limit: u32,

    /// Large file threshold (default: "1MB")
    /// Files larger than this are considered "large" and subject to additional concurrency limits
    #[serde(default = "default_large_file_threshold")]
    pub large_file_threshold: String,

    /// Enable session cleanup task (default: true)
    #[serde(default = "default_enable_session_cleanup")]
    pub enable_session_cleanup: bool,

    /// Cleanup task interval in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval: u64,

    /// Retention days for completed sessions (default: 7 days)
    /// Completed sessions older than this will be deleted along with their file records
    #[serde(default = "default_completed_retention_days")]
    pub completed_retention_days: u32,
}

fn default_session_timeout() -> u64 {
    3600
}
fn default_max_file_size() -> String {
    "100MB".to_string()
}
fn default_max_files() -> u32 {
    1000
}
fn default_max_concurrent_uploads() -> u32 {
    5
}
fn default_upload_concurrency_limit() -> u32 {
    50
}
fn default_large_file_concurrency_limit() -> u32 {
    10
}
fn default_large_file_threshold() -> String {
    "1MB".to_string()
}
fn default_enable_session_cleanup() -> bool {
    true
}
fn default_cleanup_interval() -> u64 {
    300
}
fn default_completed_retention_days() -> u32 {
    7
}

impl BuckConfig {
    /// Parse max_file_size string to bytes
    /// Returns the size in bytes, or an error message if parsing fails
    pub fn get_max_file_size_bytes(&self) -> Result<u64, String> {
        PackConfig::get_size_from_str(&self.max_file_size, || Ok(0)).map(|v| v as u64)
    }

    /// Parse large_file_threshold string to bytes
    /// Returns the size in bytes, or an error message if parsing fails
    pub fn get_large_file_threshold_bytes(&self) -> Result<u64, String> {
        PackConfig::get_size_from_str(&self.large_file_threshold, || Ok(0)).map(|v| v as u64)
    }
}

impl Default for BuckConfig {
    fn default() -> Self {
        Self {
            session_timeout: default_session_timeout(),
            max_file_size: default_max_file_size(),
            max_files: default_max_files(),
            max_concurrent_uploads: default_max_concurrent_uploads(),
            upload_concurrency_limit: default_upload_concurrency_limit(),
            large_file_concurrency_limit: default_large_file_concurrency_limit(),
            large_file_threshold: default_large_file_threshold(),
            enable_session_cleanup: default_enable_session_cleanup(),
            cleanup_interval: default_cleanup_interval(),
            completed_retention_days: default_completed_retention_days(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    fn check_file_permission(path: &Path) {
        let metadata = std::fs::metadata(path).expect("Failed to read metadata");
        assert!(
            !metadata.permissions().readonly(),
            "File should not be read-only"
        );
    }

    #[test]
    fn test_mega_base() {
        let base_dir = mega_base();
        std::fs::create_dir_all(&base_dir).expect("Failed to create base directory");
        assert!(base_dir.exists(), "Mega base directory should exist");
        check_file_permission(&base_dir);
    }

    #[test]
    fn test_mega_cache() {
        let cache_dir = mega_cache();
        std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
        assert!(cache_dir.exists(), "Mega cache directory should exist");
        check_file_permission(&cache_dir);
    }

    #[test]
    fn test_get_size_from_str() {
        use crate::config::PackConfig;

        assert_eq!(
            PackConfig::get_size_from_str("1MB", || Ok(1000 * 1000)).unwrap(),
            1000 * 1000
        );
        assert_eq!(
            PackConfig::get_size_from_str("2MiB", || Ok(2 * 1024 * 1024)).unwrap(),
            2 * 1024 * 1024
        );
        assert_eq!(
            PackConfig::get_size_from_str("20M", || Ok(0)).unwrap(),
            20 * 1024 * 1024
        );
        assert_eq!(
            PackConfig::get_size_from_str("3GB", || Ok(3 * 1000 * 1000 * 1000)).unwrap(),
            3 * 1000 * 1000 * 1000
        );
        assert_eq!(
            PackConfig::get_size_from_str("4GiB", || Ok(4 * 1024 * 1024 * 1024)).unwrap(),
            4 * 1024 * 1024 * 1024
        );
        assert_eq!(
            PackConfig::get_size_from_str("4G", || Ok(4 * 1024 * 1024 * 1024)).unwrap(),
            4 * 1024 * 1024 * 1024
        );
        assert_eq!(PackConfig::get_size_from_str("1%", || Ok(100)).unwrap(), 1);
        assert_eq!(
            PackConfig::get_size_from_str("50%", || Ok(100)).unwrap(),
            50
        );
        assert_eq!(
            PackConfig::get_size_from_str("0.01", || Ok(100)).unwrap(),
            1
        );
        assert_eq!(
            PackConfig::get_size_from_str("0.5", || Ok(100)).unwrap(),
            50
        );
        assert_eq!(
            PackConfig::get_size_from_str("1", || Ok(100)).unwrap(),
            1024 * 1024 * 1024
        );
    }
}
