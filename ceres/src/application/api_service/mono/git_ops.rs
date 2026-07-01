//! Git operations port used by CL application logic.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use common::errors::MegaError;
use git_internal::{hash::ObjectHash, internal::object::tree::Tree};
use jupiter::storage::Storage;

use super::service::MonoApiService;
use crate::application::api_service::ApiHandler;

/// Git tree/blob operations required by CL diff and merge flows.
#[async_trait]
pub trait GitOpsPort: ApiHandler + Send + Sync {
    fn storage(&self) -> &Storage;

    async fn traverse_tree(&self, root_tree: Tree)
    -> Result<Vec<(PathBuf, ObjectHash)>, MegaError>;

    async fn search_tree_for_update(&self, parent: &Path) -> Result<Vec<Arc<Tree>>, MegaError>;

    async fn attach_project_path_to_monorepo_root(&self, path: &str) -> Result<(), MegaError>;
}

#[async_trait]
impl GitOpsPort for MonoApiService {
    fn storage(&self) -> &Storage {
        MonoApiService::storage(self)
    }

    async fn traverse_tree(
        &self,
        root_tree: Tree,
    ) -> Result<Vec<(PathBuf, ObjectHash)>, MegaError> {
        MonoApiService::traverse_tree(self, root_tree).await
    }

    async fn search_tree_for_update(&self, parent: &Path) -> Result<Vec<Arc<Tree>>, MegaError> {
        ApiHandler::search_tree_for_update(self, parent)
            .await
            .map_err(|e| MegaError::Other(e.to_string()))
    }

    async fn attach_project_path_to_monorepo_root(&self, path: &str) -> Result<(), MegaError> {
        MonoApiService::attach_project_path_to_monorepo_root(self, path).await
    }
}
