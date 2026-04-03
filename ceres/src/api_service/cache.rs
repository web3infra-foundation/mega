use std::sync::Arc;

use common::errors::MegaError;
use git_internal::{
    hash::ObjectHash,
    internal::object::{
        commit::{ArchivedCommit, Commit},
        tree::{ArchivedTree, Tree},
    },
};
use jupiter::redis::{AsyncCommands, ConnectionManager};
use rkyv::rancor::Error;

#[derive(Clone)]
pub struct GitObjectCache {
    pub connection: ConnectionManager,
    pub prefix: String,
}

const DEFAULT_EXPIRY_SECONDS: u64 = 60 * 60 * 24; // 1 days

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
        {
            match rkyv::access::<ArchivedTree, Error>(&data) {
                Ok(archived) => {
                    let tree = rkyv::deserialize::<Tree, Error>(archived)?;
                    return Ok(Arc::new(tree));
                }
                Err(err) => {
                    tracing::error!("deserialize failed with fetch key: {:?}, err{:?}", key, err);
                }
            }
        }

        let tree_raw = fetch_tree(oid).await?;
        let serialized = rkyv::to_bytes::<Error>(&tree_raw)?;
        let tree = Arc::new(tree_raw);
        let _: () = conn
            .set_ex(key, serialized.as_slice(), DEFAULT_EXPIRY_SECONDS)
            .await?;

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
        {
            match rkyv::access::<ArchivedCommit, Error>(&data) {
                Ok(archived) => {
                    let commit = rkyv::deserialize::<Commit, Error>(archived)?;
                    return Ok(Arc::new(commit));
                }
                Err(err) => {
                    tracing::error!("deserialize failed with fetch key: {:?} err{:?}", key, err);
                }
            }
        }

        let commit_raw = fetch_commit(oid).await?;
        let serialized = rkyv::to_bytes::<Error>(&commit_raw)?;
        let commit = Arc::new(commit_raw);
        let _: () = conn
            .set_ex(key, serialized.as_slice(), DEFAULT_EXPIRY_SECONDS)
            .await?;

        Ok(commit)
    }
}
