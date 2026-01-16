use std::sync::Arc;

use common::errors::MegaError;
use git_internal::{
    hash::ObjectHash,
    internal::object::{commit::Commit, tree::Tree},
};
use redis::{AsyncCommands, aio::ConnectionManager};

#[derive(Clone)]
pub struct GitObjectCache {
    pub connection: ConnectionManager,
    pub prefix: String,
}

const DEFAULT_EXPIRY_SECONDS: u64 = 60 * 60 * 24 * 7; // 7 days

impl GitObjectCache {
    pub async fn get_tree<F, Fut>(
        &self,
        oid: ObjectHash,
        fetch_tree: F,
    ) -> Result<Arc<Tree>, MegaError>
    where
        F: Fn(ObjectHash) -> Fut,
        Fut: Future<Output = Result<Tree, MegaError>>,
    {
        let key = format!("{}:tree:{}", self.prefix, oid);
        let mut conn = self.connection.clone();

        if let Ok(data) = conn.get::<_, Vec<u8>>(&key).await
            && !data.is_empty()
            && let Ok((tree, _)) = bincode::decode_from_slice(&data, bincode::config::standard())
        {
            return Ok(Arc::new(tree));
        }

        let tree_raw = fetch_tree(oid).await?;
        let tree = Arc::new(tree_raw);

        let serialized = bincode::encode_to_vec(tree.as_ref(), bincode::config::standard())?;
        let _: () = conn.set_ex(key, serialized, DEFAULT_EXPIRY_SECONDS).await?;

        Ok(tree)
    }

    pub async fn get_commit<F, Fut>(
        &self,
        oid: ObjectHash,
        fetch_commit: F,
    ) -> Result<Arc<Commit>, MegaError>
    where
        F: Fn(ObjectHash) -> Fut,
        Fut: Future<Output = Result<Commit, MegaError>>,
    {
        let mut conn = self.connection.clone();
        let key = format!("{}:commit:{}", self.prefix, oid);

        if let Ok(data) = conn.get::<_, Vec<u8>>(&key).await
            && !data.is_empty()
            && let Ok((commit, _)) = bincode::decode_from_slice(&data, bincode::config::standard())
        {
            return Ok(Arc::new(commit));
        }

        let commit_raw = fetch_commit(oid).await?;
        let commit = Arc::new(commit_raw);

        let serialized = bincode::encode_to_vec(commit.as_ref(), bincode::config::standard())?;
        let _: () = conn.set_ex(key, serialized, DEFAULT_EXPIRY_SECONDS).await?;

        Ok(commit)
    }
}
