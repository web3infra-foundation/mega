use c::{ConfigError, FileFormat};
use config as c;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub log: LogConfig,
    pub database: DbConfig,
    pub ssh: SshConfig,
    pub storage: StorageConfig,
    pub monorepo: MonoConfig,
    pub pack: PackConfig,
    pub ztm: ZTMConfig,
    pub lfs: LFSConfig,
}

impl Config {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let builder = c::Config::builder().add_source(c::File::new(path, FileFormat::Toml));
        let config = builder.build().unwrap();

        Config::from_config(config)
    }

    pub fn from_config(config: c::Config) -> Result<Self, c::ConfigError> {
        // config.get::<Self>(env!("CARGO_PKG_NAME"))
        config.try_deserialize::<Config>()
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
            log_path: PathBuf::from("/tmp/.mega/logs"),
            level: String::from("info"),
            print_std: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbConfig {
    pub db_type: String,
    pub db_path: String,
    pub db_url: String,
    pub max_connection: u32,
    pub min_connection: u32,
    pub sqlx_logging: bool,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            db_type: String::new(),
            db_path: String::new(),
            db_url: String::new(),
            max_connection: 32,
            min_connection: 16,
            sqlx_logging: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SshConfig {
    pub ssh_key_path: PathBuf,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            ssh_key_path: PathBuf::from("/tmp/.mega/ssh"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageConfig {
    pub raw_obj_storage_type: String,
    pub big_obj_threshold: usize,
    pub raw_obj_local_path: PathBuf,
    pub lfs_obj_local_path: PathBuf,
    pub obs_access_key: String,
    pub obs_secret_key: String,
    pub obs_region: String,
    pub obs_endpoint: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            raw_obj_storage_type: String::from("LOCAL"),
            big_obj_threshold: 1024,
            raw_obj_local_path: PathBuf::from("/tmp/.mega/objects"),
            lfs_obj_local_path: PathBuf::from("/tmp/.mega/lfs"),
            obs_access_key: String::new(),
            obs_secret_key: String::new(),
            obs_region: String::from("cn-east-3"),
            obs_endpoint: String::from("https://obs.cn-east-3.myhuaweicloud.com"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MonoConfig {
    pub import_dir: PathBuf,
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            import_dir: PathBuf::from("/third-part"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackConfig {
    pub pack_decode_mem_size: usize,
    pub pack_decode_cache_path: PathBuf,
    pub clean_cache_after_decode: bool,
    pub channel_message_size: usize,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            pack_decode_mem_size: 4,
            pack_decode_cache_path: PathBuf::from("/tmp/.mega/cache"),
            clean_cache_after_decode: true,
            channel_message_size: 1_000_000,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZTMConfig {
    pub ca: String,
    pub hub: String,
    pub agent: String,
}

impl Default for ZTMConfig {
    fn default() -> Self {
        Self {
            ca: String::from("127.0.0.1:9999"),
            hub: String::from("127.0.0.1:8888"),
            agent: String::from("127.0.0.1:7777"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LFSConfig {
    pub enable_split: bool,
    #[serde(default = "default_split_size")]
    pub split_size: usize,
}

fn default_split_size() -> usize {
    1024 * 1024 * 20 // 20MB
}
