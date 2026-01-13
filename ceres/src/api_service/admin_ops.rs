//! Path-aware admin permission operations.
//!
//! This module provides admin permission checking with path context support.
//! Each root directory (e.g., project, doc, release) can have its own admin list
//! defined in its `.mega_cedar.json` file.
//!
//! # Design
//! - Admin permissions are scoped to root directories (first-level dirs under /)
//! - Each root directory has its own `.mega_cedar.json` file
//! - Cache keys are namespaced by both instance prefix and root directory
//! - Empty paths fall back to the default directory (project)

use redis::AsyncCommands;

use crate::api_service::mono_api_service::MonoApiService;
use common::errors::MegaError;
use git_internal::internal::object::tree::Tree;
use jupiter::utils::converter::FromMegaModel;

/// Cache TTL for admin lists (10 minutes).
pub const ADMIN_CACHE_TTL: u64 = 600;

/// The Cedar entity file name in each root directory.
pub const ADMIN_FILE: &str = ".mega_cedar.json";

/// Default root directory when path is empty or root.
pub const DEFAULT_ROOT_DIR: &str = "project";

/// Extract the root directory from a path.
///
/// Examples:
/// - `/project/src/main.rs` -> `project`
/// - `/doc/readme.md` -> `doc`
/// - `/` or empty -> `project` (default)
pub fn extract_root_dir(path: &str) -> String {
    let path = path.trim_start_matches('/');
    path.split('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_ROOT_DIR)
        .to_string()
}

impl MonoApiService {
    /// Check if a user is an admin for the specified path context.
    pub async fn check_is_admin(&self, username: &str, path: &str) -> Result<bool, MegaError> {
        let root_dir = extract_root_dir(path);
        let admins = self.get_effective_admins(&root_dir).await?;
        Ok(admins.contains(&username.to_string()))
    }

    /// Retrieve all admin usernames for the specified path context.
    pub async fn get_all_admins(&self, path: &str) -> Result<Vec<String>, MegaError> {
        let root_dir = extract_root_dir(path);
        self.get_effective_admins(&root_dir).await
    }

    /// Get admins from cache or storage.
    async fn get_effective_admins(&self, root_dir: &str) -> Result<Vec<String>, MegaError> {
        if let Ok(admins) = self.get_admins_from_cache(root_dir).await {
            return Ok(admins);
        }

        let store = self.load_admin_entity_store(root_dir).await?;
        let resolver = saturn::admin_resolver::AdminResolver::from_entity_store(&store);
        let admins = resolver.admin_list();

        if let Err(e) = self.cache_admins(root_dir, &admins).await {
            tracing::warn!("Failed to write admin cache for {}: {}", root_dir, e);
        }

        Ok(admins)
    }

    /// Invalidate the admin list cache for a root directory.
    pub async fn invalidate_admin_cache(&self, root_dir: &str) {
        let mut conn = self.git_object_cache.connection.clone();
        let key = format!("{}:admin:list:{}", self.git_object_cache.prefix, root_dir);
        if let Err(e) = conn.del::<_, ()>(&key).await {
            tracing::warn!("Failed to invalidate admin cache for {}: {}", root_dir, e);
        }
    }

    /// Load EntityStore from `/{root_dir}/.mega_cedar.json`.
    async fn load_admin_entity_store(
        &self,
        root_dir: &str,
    ) -> Result<saturn::entitystore::EntityStore, MegaError> {
        let dir_path = format!("/{}", root_dir);
        let mono_storage = self.storage.mono_storage();

        let target_tree = if let Ok(Some(dir_ref)) = mono_storage.get_main_ref(&dir_path).await {
            Tree::from_mega_model(
                mono_storage
                    .get_tree_by_hash(&dir_ref.ref_tree_hash)
                    .await?
                    .ok_or_else(|| {
                        MegaError::Other(format!("Tree {} not found", dir_ref.ref_tree_hash))
                    })?,
            )
        } else {
            // Fallback: traverse from root to find the directory
            let root_ref = mono_storage
                .get_main_ref("/")
                .await?
                .ok_or_else(|| MegaError::Other("Root ref not found".into()))?;

            let root_tree = Tree::from_mega_model(
                mono_storage
                    .get_tree_by_hash(&root_ref.ref_tree_hash)
                    .await?
                    .ok_or_else(|| MegaError::Other("Root tree not found".into()))?,
            );

            let dir_item = root_tree
                .tree_items
                .iter()
                .find(|item| item.name == root_dir)
                .ok_or_else(|| MegaError::Other(format!("'{}' directory not found", root_dir)))?;

            Tree::from_mega_model(
                mono_storage
                    .get_tree_by_hash(&dir_item.id.to_string())
                    .await?
                    .ok_or_else(|| {
                        MegaError::Other(format!("Tree for '{}' not found", root_dir))
                    })?,
            )
        };

        let blob_item = target_tree
            .tree_items
            .iter()
            .find(|item| item.name == ADMIN_FILE)
            .ok_or_else(|| {
                MegaError::Other(format!("{} not found in /{}", ADMIN_FILE, root_dir))
            })?;

        let content_bytes = self
            .storage
            .git_service
            .get_object_as_bytes(&blob_item.id.to_string())
            .await?;

        let content = String::from_utf8(content_bytes)
            .map_err(|e| MegaError::Other(format!("UTF-8 decode failed: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| MegaError::Other(format!("JSON parse failed: {}", e)))
    }

    async fn get_admins_from_cache(&self, root_dir: &str) -> Result<Vec<String>, MegaError> {
        let mut conn = self.git_object_cache.connection.clone();
        let key = format!("{}:admin:list:{}", self.git_object_cache.prefix, root_dir);
        let data: Option<String> = conn.get(&key).await?;

        match data {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| MegaError::Other(format!("Parse cache failed: {}", e))),
            None => Err(MegaError::Other("Cache miss".into())),
        }
    }

    async fn cache_admins(&self, root_dir: &str, admins: &[String]) -> Result<(), MegaError> {
        let mut conn = self.git_object_cache.connection.clone();
        let json = serde_json::to_string(admins)
            .map_err(|e| MegaError::Other(format!("Serialize failed: {}", e)))?;

        let key = format!("{}:admin:list:{}", self.git_object_cache.prefix, root_dir);
        conn.set_ex::<_, _, ()>(&key, json, ADMIN_CACHE_TTL).await?;
        Ok(())
    }
}
