use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use git_internal::{
    errors::GitError,
    hash::SHA1,
    internal::object::{commit::Commit, tree::Tree},
};

use crate::api_service::ApiHandler;

#[derive(Debug, Default, Clone)]
pub struct GitObjectCache {
    trees: HashMap<SHA1, Arc<Tree>>,
    commits: HashMap<SHA1, Arc<Commit>>,
}

impl GitObjectCache {
    pub fn new() -> GitObjectCache {
        GitObjectCache::default()
    }
}

pub async fn get_tree_from_cache<R: ApiHandler + ?Sized>(
    repo: &R,
    oid: SHA1,
    cache: &Arc<Mutex<GitObjectCache>>,
) -> Result<Arc<Tree>, GitError> {
    {
        let guard = cache.lock().await;
        if let Some(tree) = guard.trees.get(&oid) {
            return Ok(tree.clone());
        }
    }

    let tree = Arc::new(repo.get_tree_by_hash(&oid.to_string()).await);
    {
        let mut guard = cache.lock().await;
        guard.trees.insert(oid, tree.clone());
    }
    Ok(tree)
}

pub async fn get_commit_from_cache<R: ApiHandler + ?Sized>(
    repo: &R,
    oid: SHA1,
    cache: &Arc<Mutex<GitObjectCache>>,
) -> Result<Arc<Commit>, GitError> {
    {
        let guard = cache.lock().await;
        if let Some(commit) = guard.commits.get(&oid) {
            return Ok(commit.clone());
        }
    }

    let commit: Arc<Commit> = Arc::new(repo.get_commit_by_hash(&oid.to_string()).await);

    {
        let mut guard = cache.lock().await;
        guard.commits.insert(oid, commit.clone());
    }
    Ok(commit)
}
