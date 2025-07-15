use clap::Parser;

use crate::internal::config;

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Add a configuration entry to database
    #[clap(long, group("mode"), requires("valuepattern"))]
    pub add: bool,
    /// Get a single configuration entry that satisfied key and value pattern from database
    #[clap(long, group("mode"))]
    pub get: bool,
    /// Get all configuration entries that satisfied key and value pattern from database
    #[clap(long("get-all"), group("mode"))]
    pub get_all: bool,
    /// Remove a single configuration entry from database
    #[clap(long, group("mode"))]
    pub unset: bool,
    /// Remove all the configuration entries that satisfied key and valuepattern from database
    #[clap(long("unset-all"), group("mode"))]
    pub unset_all: bool,
    /// List all the configuration entries from database
    #[clap(long, short, group("mode"))]
    pub list: bool,
    /// If set, only print the key string of the configuration entry instead of the key=value.
    /// This is only valid when `list` is set.
    #[clap(long("name-only"), requires = "list")]
    pub name_only: bool,
    /// The key string of the configuration entry, should be like configuration.[name].key
    #[clap(value_name("key"), required_unless_present("list"))]
    pub key: Option<String>,
    /// the value or the possible value pattern of the configuration entry
    #[clap(value_name("value_pattern"), required_unless_present("mode"))]
    pub valuepattern: Option<String>,
    /// If the target key is not present, return the given default value.
    /// This is only valid when `get` is set.
    #[clap(long, short = 'd', requires = "get")]
    pub default: Option<String>,
}

impl ConfigArgs {
    pub fn validate(&self) -> Result<(), String> {
        // validate the default value is only present when get is set
        if self.default.is_some() && !(self.get || self.get_all) {
            return Err("default value is only valid when get (get_all) is set".to_string());
        }
        // validate that name_only is only valid when list is set
        if self.name_only && !self.list {
            return Err("--name-only is only valid when --list is set".to_string());
        }

        Ok(())
    }
}

pub struct Key {
    configuration: String,
    name: Option<String>,
    key: String,
}

pub async fn execute(args: ConfigArgs) {
    if let Err(e) = args.validate() {
        eprintln!("error: {e}");
        return;
    }
    if args.list {
        list_config(args.name_only).await;
    } else {
        let origin_key = args.key.unwrap();
        let key: Key = parse_key(origin_key).await;
        if args.add {
            add_config(&key, &args.valuepattern.unwrap()).await;
        } else if args.get {
            get_config(&key, args.default.as_deref(), args.valuepattern.as_deref()).await;
        } else if args.get_all {
            get_all_config(&key, args.default.as_deref(), args.valuepattern.as_deref()).await;
        } else if args.unset {
            unset_config(&key, args.valuepattern.as_deref()).await;
        } else if args.unset_all {
            unset_all_config(&key, args.valuepattern.as_deref()).await;
        } else {
            // If none of the above flags are present, then default to setting a config
            set_config(&key, &args.valuepattern.unwrap()).await;
        }
    }
}

/// Parse the original key string to three fields: configuration, name and key
/// The parsing strategy for the three parameters configuration, name, and key is as follows:
/// If the original key parameter string does not contain a . symbol, an error is directly raised.
/// If the original key parameter string contains exactly one . symbol, the entire key parameter string is parsed as configuration.key.
/// If the original key parameter string contains more than one . symbol, the entire key parameter string is parsed as configuration.name.key, where the two . symbols correspond to the first . and the last . in the original parameter string.
async fn parse_key(mut origin_key: String) -> Key {
    let configuration: String;
    let name: Option<String>;
    (configuration, origin_key) = match origin_key.split_once(".") {
        Some((first_part, remainer)) => (first_part.to_string(), remainer.to_string()),
        None => {
            panic!("error: key does not contain a section: {origin_key}");
        }
    };
    (name, origin_key) = match origin_key.rsplit_once(".") {
        Some((first_part, remainer)) => (Some(first_part.to_string()), remainer.to_string()),
        None => (None, origin_key),
    };
    let key: String = origin_key;
    Key {
        configuration,
        name,
        key,
    }
}

/// Add a configuration entry by the given key and value (create new one no matter old one is present or not)
async fn add_config(key: &Key, value: &str) {
    config::Config::insert(&key.configuration, key.name.as_deref(), &key.key, value).await;
}

/// Set a configuration entry by the given key and value (if old one is present, overwrites its value, otherwise create new one)
async fn set_config(key: &Key, value: &str) {
    // First, check whether given key has multiple values
    let values: Vec<String> =
        config::Config::get_all(&key.configuration, key.name.as_deref(), &key.key).await;
    if values.len() >= 2 {
        eprintln!(
            "warning: {}.{} has multiple values",
            &key.configuration,
            match &key.name {
                Some(str) => str.to_string() + ".",
                None => "".to_string(),
            } + &key.key
        );
        eprintln!("error: cannot overwrite multiple values with a single value");
    } else if values.len() == 1 {
        config::Config::update(&key.configuration, key.name.as_deref(), &key.key, value).await;
    } else {
        config::Config::insert(&key.configuration, key.name.as_deref(), &key.key, value).await;
    }
}

/// Get the first configuration by the given key and value pattern
async fn get_config(key: &Key, default: Option<&str>, valuepattern: Option<&str>) {
    let value: Option<String> =
        config::Config::get(&key.configuration, key.name.as_deref(), &key.key).await;
    if let Some(v) = value {
        if let Some(vp) = valuepattern {
            // if value pattern is present, check it
            if v.contains(vp) {
                println!("{v}");
            }
        } else {
            // if value pattern is not present, just print it
            println!("{v}");
        }
    } else if let Some(default_value) = default {
        // if value is not exits just return the default value if it's present
        println!("{default_value}");
    }
}

/// Get all the configurations by the given key and value pattern
async fn get_all_config(key: &Key, default: Option<&str>, valuepattern: Option<&str>) {
    let values: Vec<String> =
        config::Config::get_all(&key.configuration, key.name.as_deref(), &key.key).await;
    let mut matched_any = false;
    for value in values {
        if let Some(vp) = valuepattern {
            // for each value, check if it matches the pattern
            if value.contains(vp) {
                println!("{value}");
                matched_any = true;
            }
        } else {
            // print all if value pattern is not present
            matched_any = true;
            println!("{value}");
        }
    }
    if !matched_any {
        if let Some(default_value) = default {
            // if no value matches the pattern, print the default value if it's present
            println!("{default_value}");
        }
    }
}

/// Remove one configuration by given key and value pattern
async fn unset_config(key: &Key, valuepattern: Option<&str>) {
    config::Config::remove_config(
        &key.configuration,
        key.name.as_deref(),
        &key.key,
        valuepattern,
        false,
    )
    .await;
}

/// Remove all configurations by given key and value pattern
async fn unset_all_config(key: &Key, valuepattern: Option<&str>) {
    config::Config::remove_config(
        &key.configuration,
        key.name.as_deref(),
        &key.key,
        valuepattern,
        true,
    )
    .await;
}

/// List all configurations
async fn list_config(name_only: bool) {
    let configurations = config::Config::list_all().await;
    for (key, value) in configurations {
        // If name_only is set, only print the key string
        // Otherwise, print the key=value pair
        if name_only {
            println!("{key}");
        } else {
            println!("{key}={value}");
        }
    }
}
