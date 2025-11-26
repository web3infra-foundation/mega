use std::sync::Arc;

use redis::{AsyncCommands, Client};

use common::errors::MegaError;
use git_internal::{
    hash::SHA1,
    internal::object::{commit::Commit, tree::Tree},
};

#[derive(Clone)]
pub struct GitObjectCache {
    pub redis: Arc<Client>,
    pub prefix: String,
}

const DEFAULT_EXPIRY_SECONDS: u64 = 60 * 60 * 24 * 7; // 7 days

impl GitObjectCache {
    pub fn mock() -> Self {
        let redis_client = Arc::new(Client::open("redis://127.0.0.1:6379".to_string()).unwrap());
        GitObjectCache {
            redis: redis_client,
            prefix: "".to_string(),
        }
    }

    pub async fn ping(&self) -> Result<(), MegaError> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let _: () = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn get_tree<F, Fut>(&self, oid: SHA1, fetch_tree: F) -> Result<Arc<Tree>, MegaError>
    where
        F: Fn(SHA1) -> Fut,
        Fut: Future<Output = Result<Tree, MegaError>>,
    {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let key = format!("{}:tree:{}", self.prefix, oid);

        if let Ok(json) = conn.get::<_, String>(&key).await
            && !json.is_empty()
            && let Ok(tree) = serde_json::from_str::<Tree>(&json)
        {
            return Ok(Arc::new(tree));
        }

        let tree_raw = fetch_tree(oid).await?;
        let tree = Arc::new(tree_raw);

        let serialized = serde_json::to_string(&*tree)?;
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
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let key = format!("{}:commit:{}", self.prefix, oid);

        if let Ok(json) = conn.get::<_, String>(&key).await
            && !json.is_empty()
            && let Ok(commit) = serde_json::from_str::<Commit>(&json)
        {
            return Ok(Arc::new(commit));
        }

        let commit_raw = fetch_commit(oid).await?;
        let commit = Arc::new(commit_raw);

        let serialized = serde_json::to_string(&*commit)?;
        let _: () = conn.set_ex(key, serialized, DEFAULT_EXPIRY_SECONDS).await?;

        Ok(commit)
    }
}
