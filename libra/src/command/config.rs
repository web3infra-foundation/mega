use clap::Parser;

use crate::internal::config;

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Add a configuration entry to database
    #[clap(long, group("mode"), requires("valuepattern"))]
    add: bool,
    /// Get a single configuration entry that satisfied key and value pattern from database
    #[clap(long, group("mode"))]
    get: bool,
    /// Get all configuration entries that satisfied key and value pattern from database
    #[clap(long("get-all"), group("mode"))]
    get_all: bool,
    /// Remove a single configuration entry from database
    #[clap(long, group("mode"))]
    unset: bool,
    /// Remove all the configuration entries that satisfied key and valuepattern from database
    #[clap(long("unset-all"), group("mode"))]
    unset_all: bool,
    /// List all the configuration entries from database
    #[clap(long, short, group("mode"))]
    list: bool,
    /// The key string of the configuration entry, should be like configuration.[name].key
    #[clap(value_name("key"), required_unless_present("list"))]
    key: Option<String>,
    /// the value or the possible value pattern of the configuration entry
    #[clap(value_name("value_pattern"), required_unless_present("mode"))]
    valuepattern: Option<String>,
}

pub struct Key {
    configuration: String,
    name: Option<String>,
    key: String,
}

pub async fn execute(args: ConfigArgs) {
    if args.list {
        list_config().await;
    } else {
        let origin_key = args.key.unwrap();
        let key: Key = parse_key(origin_key).await;
        if args.add {
            add_config(&key, &args.valuepattern.unwrap()).await;
        } else if args.get {
            get_config(&key, args.valuepattern.as_deref()).await;
        } else if args.get_all {
            get_all_config(&key, args.valuepattern.as_deref()).await;
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
            panic!("error: key does not contain a section: {}", origin_key);
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
async fn get_config(key: &Key, valuepattern: Option<&str>) {
    let value: Option<String> =
        config::Config::get(&key.configuration, key.name.as_deref(), &key.key).await;
    if let Some(v) = value {
        if let Some(vp) = valuepattern {
            // if value pattern is present, check it
            if v.contains(vp) {
                println!("{}", v);
            }
        } else {
            // if value pattern is not present, just print it
            println!("{}", v);
        }
    }
}

/// Get all the configurations by the given key and value pattern
async fn get_all_config(key: &Key, valuepattern: Option<&str>) {
    let values: Vec<String> =
        config::Config::get_all(&key.configuration, key.name.as_deref(), &key.key).await;
    for value in values {
        if let Some(vp) = valuepattern {
            // for each value, check if it matches the pattern
            if value.contains(vp) {
                println!("{}", value)
            }
        } else {
            // print all if value pattern is not present
            println!("{}", value)
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
async fn list_config() {
    let configurations = config::Config::list_all().await;
    for (key, value) in configurations {
        println!("{}={}", key, value);
    }
}
