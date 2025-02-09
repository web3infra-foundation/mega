mod backend;
mod frontend;

use common::config::Config as MegaConfig;
use tracing::{warn, warn_span};

const CONFIG_PATH: &str = "config.toml";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let span = warn_span!("Init");
    let _enter = span.enter();

    let config = MegaConfig::new(CONFIG_PATH).unwrap_or_else(|e| {
        warn!("Error loading config file: {e}, using default values");
        MegaConfig::default()
    });

    // TODO: return a result and handle it.
    backend::init(&config).await;
    frontend::init(&config).await;
}

