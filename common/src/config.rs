use c::{ConfigError, FileFormat};
use config as c;
use config::builder::DefaultState;
use config::{Source, ValueKind};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub base_dir: PathBuf,
    pub log: LogConfig,
    pub database: DbConfig,
    pub storage: StorageConfig,
    pub monorepo: MonoConfig,
    pub pack: PackConfig,
    pub lfs: LFSConfig,
    // Not used in mega app
    #[serde(default)]
    pub oauth: Option<OauthConfig>,
}

impl Config {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let builder = c::Config::builder()
            .add_source(c::File::new(path, FileFormat::Toml))
            .add_source(c::Environment::with_prefix("mega")); // e.g. MEGA_BASE_DIR == base_dir
                                                              // support ${} variable substitution
        let config = variable_placeholder_substitute(builder);

        Config::from_config(config)
    }

    pub fn from_config(config: c::Config) -> Result<Self, c::ConfigError> {
        // config.get::<Self>(env!("CARGO_PKG_NAME"))
        config.try_deserialize::<Config>()
    }
}

impl Default for Config {
    fn default() -> Self {
        let base_dir = PathBuf::from(
            std::env::var("MEGA_BASE_DIR").unwrap_or_else(|_| "/tmp/.mega".to_string()),
        );
        std::fs::create_dir_all(&base_dir).unwrap();

        // use mega/config.toml because mega use sqlite as default db
        let default_config = include_str!("../../mega/config.toml");
        let default_config = default_config
            .lines()
            .map(|line| {
                if line.starts_with("base_dir ") {
                    format!("base_dir = \"{}\"", base_dir.to_str().unwrap())
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
            println!("{}: {} -> {}", k, str, &new_str);
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
                    format!("{}.{}", key, k)
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
            db_type: String::from("sqlite"),
            db_path: String::from("/tmp/.mega/mega.db"),
            db_url: String::from("postgres://mega:mega@localhost:5432/mega"),
            max_connection: 32,
            min_connection: 16,
            sqlx_logging: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageConfig {
    pub obs_access_key: String,
    pub obs_secret_key: String,
    pub obs_region: String,
    pub obs_endpoint: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
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
    pub disable_http_push: bool,
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            import_dir: PathBuf::from("/third-part"),
            disable_http_push: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackConfig {
    pub pack_decode_mem_size: usize,
    pub pack_decode_cache_path: PathBuf,
    pub clean_cache_after_decode: bool,
    pub channel_message_size: usize,
    pub maximum_pack_size: usize,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            pack_decode_mem_size: 4,
            pack_decode_cache_path: PathBuf::from("/tmp/.mega/cache"),
            clean_cache_after_decode: true,
            channel_message_size: 1_000_000,
            maximum_pack_size: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LFSConfig {
    pub url: String,
    pub lfs_obj_local_path: PathBuf,
    pub enable_split: bool,
    pub split_size: usize,
}

impl Default for LFSConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8000".to_string(),
            lfs_obj_local_path: PathBuf::from("/tmp/.mega/lfs"),
            enable_split: true,
            split_size: 1024 * 1024 * 1024,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OauthConfig {
    pub github_client_id: String,
    pub github_client_secret: String,
    pub ui_domain: String,
    pub cookie_domain: String,
}
