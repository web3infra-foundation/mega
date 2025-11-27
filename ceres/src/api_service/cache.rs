use std::sync::Arc;

use jupiter::redis::client::RedisPoolClient;
use redis::AsyncCommands;

use common::errors::MegaError;
use git_internal::{
    hash::SHA1,
    internal::object::{commit::Commit, tree::Tree},
};

#[derive(Clone)]
pub struct GitObjectCache {
    pub redis: Arc<RedisPoolClient>,
    pub prefix: String,
}

const DEFAULT_EXPIRY_SECONDS: u64 = 60 * 60 * 24 * 7; // 7 days

impl GitObjectCache {
    pub fn mock() -> Self {
        GitObjectCache {
            redis: Arc::new(RedisPoolClient::mock()),
            prefix: "mock:key".to_string(),
        }
    }

    pub async fn ping(&self) -> Result<(), MegaError> {
        let mut conn = self.redis.get_connection().await?;
        let _: () = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn get_tree<F, Fut>(&self, oid: SHA1, fetch_tree: F) -> Result<Arc<Tree>, MegaError>
    where
        F: Fn(SHA1) -> Fut,
        Fut: Future<Output = Result<Tree, MegaError>>,
    {
        let mut conn = self.redis.get_connection().await?;
        let key = format!("{}:tree:{}", self.prefix, oid);

        if let Ok(json) = conn.get::<_, Vec<u8>>(&key).await
            && !json.is_empty()
            && let Ok((tree, _)) = bincode::decode_from_slice(&json, bincode::config::standard())
        {
            return Ok(Arc::new(tree));
        }

        let tree_raw = fetch_tree(oid).await?;
        let tree = Arc::new(tree_raw);

        let serialized = bincode::encode_to_vec(&tree, bincode::config::standard())?;
        let _: () = conn.set_ex(key, serialized, DEFAULT_EXPIRY_SECONDS).await?;

        Ok(tree)
    }

    pub async fn get_commit<F, Fut>(
        &self,
        oid: SHA1,
        fetch_commit: F,
    ) -> Result<Arc<Commit>, MegaError>
    where
        F: Fn(SHA1) -> Fut,
        Fut: Future<Output = Result<Commit, MegaError>>,
    {
        let mut conn = self.redis.get_connection().await?;
        let key = format!("{}:commit:{}", self.prefix, oid);

        if let Ok(json) = conn.get::<_, Vec<u8>>(&key).await
            && !json.is_empty()
            && let Ok((commit, _)) = bincode::decode_from_slice(&json, bincode::config::standard())
        {
            return Ok(Arc::new(commit));
        }

        let commit_raw = fetch_commit(oid).await?;
        let commit = Arc::new(commit_raw);

        let serialized = bincode::encode_to_vec(&commit, bincode::config::standard())?;
        let _: () = conn.set_ex(key, serialized, DEFAULT_EXPIRY_SECONDS).await?;

        Ok(commit)
    }
}
