//! # Configuration Module for Monobean
//!
//! This module defines constants and utilities for application configuration.
//!
//! ## Constants Categories
//!
//! - **Application metadata**: Version and online resources
//! - **Runtime configurations**: Application identifiers and file paths
//!
//! ## Utilities
//!
//! Provides helper macros for retrieving settings from configuration sources.
//!

use std::path::PathBuf;

use adw::gio::Settings;
use gtk::{
    gio::{self, ResourceLookupFlags},
    prelude::*,
};

use crate::core::CoreConfigChanged;

/* Application metadata */
pub const VERSION: &str = "0.0.1";
pub const WEBSITE: &str = "https://github.com/web3infra-foundation/mega";

/* Runtime configurations */
pub const APP_ID: &str = "org.Web3Infrastructure.Monobean";
pub const APP_NAME: &str = "Monobean";
pub const PREFIX: &str = "/org/Web3Infrastructure/Monobean";
pub const MEGA_CONFIG_PATH: &str = "/org/Web3Infrastructure/Monobean/mega/config.toml";

/* Helper functions for mega configs */

/// A macro that retrieves a value from GSettings with type checking and conversion.
///
/// # Arguments
///
/// * `$settings` - A GSettings object reference
/// * `$key` - The settings key to retrieve
/// * `$type` - The Rust type to convert the value to
///
/// # Returns
///
/// The value from GSettings converted to the specified Rust type.
///
/// # Panics
///
/// This macro will panic if:
/// - The setting doesn't exist in the schema
/// - The value can't be converted to the specified type
///
/// # Examples
///
/// ```
/// let settings = gio::Settings::new(APP_ID);
/// let http_port: u32 = get_setting!(settings, "print-std", u32);
/// let log_level: String = get_setting!(settings, "log-level", String);
/// let print_std: bool = get_setting!(settings, "print-std", bool);
/// ```
#[macro_export]
macro_rules! get_setting {
    ($settings:expr, $key:expr, $type:ty) => {
        match std::any::type_name::<$type>() {
            "u32" => $settings.uint($key).to_value().get::<$type>().unwrap(),
            "u64" => $settings.uint64($key).to_value().get::<$type>().unwrap(),
            "i32" => $settings.int($key).to_value().get::<$type>().unwrap(),
            "i64" => $settings.int64($key).to_value().get::<$type>().unwrap(),
            "bool" => $settings.boolean($key).to_value().get::<$type>().unwrap(),
            "alloc::string::String" => $settings.string($key).to_value().get::<$type>().unwrap(),
            _ => $settings.value($key).get::<$type>().unwrap(),
        }
    };
}

/// Retrieves the base directory path for Monobean
///
/// The directory is determined in the following priority order:
/// 1. Uses the `MONOBEAN_BASE_DIR` environment variable if set
/// 2. Falls back to system default paths when environment variable is not set:
///     - On Linux: `~/.local/share/mega/monobean`
///     - On Windows: `C:\Users\{UserName}\AppData\Local\mega\monobean`
///     - On macOS: `~/Library/Application Support/mega/monobean`
///
/// # Returns
/// A PathBuf containing the base directory path
///
/// # Panics
/// Will panic if both conditions occur:
/// - Environment variable is not set
/// - System base directories cannot be determined
///
pub fn monobean_base() -> PathBuf {
    // Get the base directory from the environment variable or use the default
    std::env::var("MONOBEAN_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| common::config::mega_base().join("monobean"))
}

/// Retrieves the cache directory path for Monobean
///
/// The directory is determined in the following priority order:
/// 1. Uses the `MONOBEAN_CACHE_DIR` environment variable if set
/// 2. Falls back to system default paths when environment variable is not set:
///     - On Linux: `~/.cache/mega/monobean`
///     - On Windows: `C:\Users\{username}\AppData\Local\Cache\mega\monobean`
///     - On macOS: `~/Library/Caches/mega/monobean`
///
/// # Returns
/// A PathBuf containing the cache directory path
///
/// # Panics
/// Will panic if both conditions occur:
/// - Environment variable is not set
/// - System cache directories cannot be determined
///
pub fn monobean_cache() -> PathBuf {
    // Get the cache directory from the environment variable or use the default
    std::env::var("MONOBEAN_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| common::config::mega_cache().join("monobean"))
}

/// TODO: So ugly...
/// - We should update build.rs and use proc macros to generate this code. - @yyk808 2025-03-12
/// - Maybe we can use dagrs to orchestrate the generation of this code. - @genedna 2025-05-15
pub fn config_update(setting: &Settings) -> Vec<CoreConfigChanged> {
    let mut update = Vec::new();
    // First, let's extract all settings and compare with defaults

    // Base settings
    let base_dir: String = get_setting!(setting, "base-dir", String);
    if !base_dir.is_empty() {
        update.push(CoreConfigChanged::BaseDir(
            base_dir.parse::<PathBuf>().unwrap(),
        ));
    }

    // Log settings
    let log_path: String = get_setting!(setting, "log-path", String);
    if !log_path.is_empty() {
        update.push(CoreConfigChanged::LogPath(
            log_path.parse::<PathBuf>().unwrap(),
        ));
    }

    let log_level: String = get_setting!(setting, "log-level", String);
    if log_level != "info" {
        update.push(CoreConfigChanged::Level(log_level));
    }

    let print_std: bool = get_setting!(setting, "print-std", bool);
    if !print_std {
        // Default is true
        update.push(CoreConfigChanged::PrintStd(print_std));
    }

    // Database settings
    let db_type: String = get_setting!(setting, "db-type", String);
    if db_type != "sqlite" {
        update.push(CoreConfigChanged::DbType(db_type));
    }

    let db_path: String = get_setting!(setting, "db-path", String);
    if !db_path.is_empty() {
        update.push(CoreConfigChanged::DbPath(
            db_path.parse::<PathBuf>().unwrap(),
        ));
    }

    let db_url: String = get_setting!(setting, "db-url", String);
    if db_url != "postgres://mono:mono@localhost:5432/mono" {
        update.push(CoreConfigChanged::DbUrl(db_url));
    }

    let max_connections: u32 = get_setting!(setting, "max-connections", u32);
    if max_connections != 16 {
        update.push(CoreConfigChanged::MaxConnection(max_connections));
    }

    let min_connections: u32 = get_setting!(setting, "min-connections", u32);
    if min_connections != 8 {
        update.push(CoreConfigChanged::MinConnection(min_connections));
    }

    let sqlx_logging: bool = get_setting!(setting, "sqlx-logging", bool);
    if sqlx_logging {
        // Default is false
        update.push(CoreConfigChanged::SqlxLogging(sqlx_logging));
    }

    // Monorepo settings
    let import_dir: String = get_setting!(setting, "import-dir", String);
    if import_dir != "/third-party" {
        update.push(CoreConfigChanged::ImportDir(
            import_dir.parse::<PathBuf>().unwrap(),
        ));
    }

    let admin: String = get_setting!(setting, "admin", String);
    if admin != "admin" {
        update.push(CoreConfigChanged::Admin(admin));
    }

    let root_dirs: String = get_setting!(setting, "root-dirs", String);
    if root_dirs != "third-party, project, doc, release" {
        // Convert comma-separated string to Vec<String>
        let dirs: Vec<String> = root_dirs.split(',').map(|s| s.trim().to_string()).collect();
        update.push(CoreConfigChanged::RootDirs(dirs));
    }

    // Authentication settings
    let http_auth: bool = get_setting!(setting, "http-auth", bool);
    if http_auth {
        // Default is false
        update.push(CoreConfigChanged::EnableHttpAuth(http_auth));
    }

    let test_user: bool = get_setting!(setting, "test-user", bool);
    if !test_user {
        // Default is true
        update.push(CoreConfigChanged::EnableTestUser(test_user));
    }

    let test_user_name: String = get_setting!(setting, "test-user-name", String);
    if test_user_name != "mega" {
        update.push(CoreConfigChanged::TestUserName(test_user_name));
    }

    let test_user_token: String = get_setting!(setting, "test-user-token", String);
    if test_user_token != "mega" {
        update.push(CoreConfigChanged::TestUserToken(test_user_token));
    }

    // Pack settings
    let pack_decode_mem_size: String = get_setting!(setting, "pack-decode-mem-size", String);
    if pack_decode_mem_size != "4G" {
        update.push(CoreConfigChanged::PackDecodeMemSize(pack_decode_mem_size));
    }

    let pack_decode_disk_size: String = get_setting!(setting, "pack-decode-disk-size", String);
    if pack_decode_disk_size != "20%" {
        update.push(CoreConfigChanged::PackDecodeDiskSize(pack_decode_disk_size));
    }

    let pack_decode_cache_path: String = get_setting!(setting, "pack-decode-cache-path", String);
    if !pack_decode_cache_path.is_empty() {
        update.push(CoreConfigChanged::PackDecodeCachePath(
            pack_decode_cache_path.parse::<PathBuf>().unwrap(),
        ));
    }

    let clean_cache: bool = get_setting!(setting, "clean-cache", bool);
    if !clean_cache {
        // Default is true
        update.push(CoreConfigChanged::CleanCacheAfterDecode(clean_cache));
    }

    let channel_message_size: u32 = get_setting!(setting, "channel-message-size", u32);
    if channel_message_size != 1000000 {
        update.push(CoreConfigChanged::ChannelMessageSize(
            channel_message_size as usize,
        ));
    }

    // LFS settings
    let lfs_url: String = get_setting!(setting, "lfs-url", String);
    if lfs_url != "http://localhost:8000" {
        update.push(CoreConfigChanged::LfsUrl(lfs_url));
    }

    // OAuth settings
    let github_client_id: String = get_setting!(setting, "github-client-id", String);
    if !github_client_id.is_empty() {
        update.push(CoreConfigChanged::GithubClientId(github_client_id));
    }

    let github_client_secret: String = get_setting!(setting, "github-client-secret", String);
    if !github_client_secret.is_empty() {
        update.push(CoreConfigChanged::GithubClientSecret(github_client_secret));
    }

    let ui_domain: String = get_setting!(setting, "ui-domain", String);
    if ui_domain != "http://localhost:3000" {
        update.push(CoreConfigChanged::UiDomain(ui_domain));
    }

    let cookie_domain: String = get_setting!(setting, "cookie-domain", String);
    if cookie_domain != "localhost" {
        update.push(CoreConfigChanged::CookieDomain(cookie_domain));
    }

    update
}

pub fn load_mega_resource(path: &str) -> Vec<u8> {
    let bytes = gio::resources_lookup_data(path, ResourceLookupFlags::all()).unwrap();
    bytes.as_ref().into()
}
