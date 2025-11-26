//! Redis client with connection pooling support.
//!
//! This module provides a pooled Redis client implementation using `deadpool-redis`.
//! Connection pooling improves performance and resource management for concurrent
//! Redis operations in the monorepo.
//!
use common::{config::RedisConfig, errors::MegaError};
use deadpool_redis::Runtime;

/// A Redis client with connection pooling support.
///
/// This client uses `deadpool-redis` to maintain a pool of connections,
/// improving performance and resource management for concurrent operations.
#[derive(Clone)]
pub struct RedisPoolClient {
    pool: deadpool_redis::Pool,
}

impl RedisPoolClient {
    /// Creates a new Redis pool client from the given configuration.
    ///
    /// # Arguments
    /// * `config` - Redis configuration including the connection URL
    pub fn new(config: &RedisConfig) -> Self {
        let cfg = deadpool_redis::Config::from_url(config.url.as_str());
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");
        Self { pool }
    }

    /// Creates a mock Redis client for testing.
    ///
    /// This connects to a local Redis instance at `redis://127.0.0.1:6379`.
    /// Panics if the pool cannot be created.
    pub fn mock() -> Self {
        let cfg = deadpool_redis::Config::from_url("redis://127.0.0.1:6379");
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("mock Redis pool creation should not fail");
        Self { pool }
    }

    pub async fn get_connection(&self) -> Result<deadpool_redis::Connection, MegaError> {
        Ok(self.pool.get().await?)
    }
}
