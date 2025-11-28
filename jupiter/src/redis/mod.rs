pub mod lock;

use common::config::RedisConfig;
use redis::aio::ConnectionManager;

/// Initializes a Redis multiplexed asynchronous connection from the given configuration.
///
/// # Arguments
/// * `config` - Redis configuration including the connection URL
pub async fn init_connection(config: &RedisConfig) -> ConnectionManager {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let client = redis::Client::open(config.url.as_str()).expect("can't open redis url");
    ConnectionManager::new(client)
        .await
        .unwrap_or_else(|_| panic!("Failed to connect to Redis at {}, please check your redis server is running and the url is correct", config.url))
}
