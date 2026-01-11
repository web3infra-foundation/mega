use redis::AsyncCommands;

use crate::api_service::mono_api_service::MonoApiService;
use common::errors::MegaError;
use git_internal::internal::object::tree::Tree;
use jupiter::utils::converter::FromMegaModel;

// Admin permission constants
pub const ADMIN_CACHE_TTL: u64 = 600;
pub const ADMIN_DIR: &str = "/project";
pub const ADMIN_DIR_NAME: &str = "project";
pub const ADMIN_FILE: &str = ".mega_cedar.json";
pub const ADMIN_CONFIG_PATH: &str = "project/.mega_cedar.json";

impl MonoApiService {
    /// Check if a user is an admin, with cache-first strategy.
    pub async fn check_is_admin(&self, username: &str) -> Result<bool, MegaError> {
        if let Ok(admins) = self.get_admins_from_cache().await {
            return Ok(admins.contains(&username.to_string()));
        }

        let store = self.load_admin_entity_store().await?;
        let resolver = saturn::admin_resolver::AdminResolver::from_entity_store(&store);
        let admins = resolver.admin_list();

        if let Err(e) = self.cache_admins(&admins).await {
            tracing::warn!("Failed to write admin cache: {}", e);
        }

        Ok(resolver.is_admin(username))
    }

    /// Retrieve all admin usernames.
    pub async fn get_all_admins(&self) -> Result<Vec<String>, MegaError> {
        if let Ok(admins) = self.get_admins_from_cache().await {
            return Ok(admins);
        }

        let store = self.load_admin_entity_store().await?;
        let resolver = saturn::admin_resolver::AdminResolver::from_entity_store(&store);
        let admins = resolver.admin_list();

        if let Err(e) = self.cache_admins(&admins).await {
            tracing::warn!("Failed to write admin cache: {}", e);
        }

        Ok(admins)
    }

    /// Delete admin list from Redis cache.
    pub async fn invalidate_admin_cache(&self) {
        let mut conn = self.git_object_cache.connection.clone();
        let key = format!("{}:admin:list", self.git_object_cache.prefix);
        if let Err(e) = conn.del::<_, ()>(&key).await {
            tracing::warn!("Failed to invalidate admin cache: {}", e);
        }
    }

    /// Load EntityStore from `/project/.mega_cedar.json`.
    pub async fn load_admin_entity_store(
        &self,
    ) -> Result<saturn::entitystore::EntityStore, MegaError> {
        let mono_storage = self.storage.mono_storage();

        let project_tree = if let Ok(Some(project_ref)) = mono_storage.get_main_ref(ADMIN_DIR).await
        {
            Tree::from_mega_model(
                mono_storage
                    .get_tree_by_hash(&project_ref.ref_tree_hash)
                    .await?
                    .ok_or_else(|| {
                        MegaError::Other(format!("Tree {} not found", project_ref.ref_tree_hash))
                    })?,
            )
        } else {
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

            let project_item = root_tree
                .tree_items
                .iter()
                .find(|item| item.name == ADMIN_DIR_NAME)
                .ok_or_else(|| {
                    MegaError::Other(format!("'{}' directory not found", ADMIN_DIR_NAME))
                })?;

            Tree::from_mega_model(
                mono_storage
                    .get_tree_by_hash(&project_item.id.to_string())
                    .await?
                    .ok_or_else(|| MegaError::Other("Project tree not found".into()))?,
            )
        };

        let blob_item = project_tree
            .tree_items
            .iter()
            .find(|item| item.name == ADMIN_FILE)
            .ok_or_else(|| MegaError::Other(format!("{} not found", ADMIN_FILE)))?;

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

    async fn get_admins_from_cache(&self) -> Result<Vec<String>, MegaError> {
        let mut conn = self.git_object_cache.connection.clone();
        let key = format!("{}:admin:list", self.git_object_cache.prefix);
        let data: Option<String> = conn.get(&key).await?;

        match data {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| MegaError::Other(format!("Parse cache failed: {}", e))),
            None => Err(MegaError::Other("Cache miss".into())),
        }
    }

    async fn cache_admins(&self, admins: &[String]) -> Result<(), MegaError> {
        let mut conn = self.git_object_cache.connection.clone();
        let json = serde_json::to_string(admins)
            .map_err(|e| MegaError::Other(format!("Serialize failed: {}", e)))?;

        let key = format!("{}:admin:list", self.git_object_cache.prefix);
        conn.set_ex::<_, _, ()>(&key, json, ADMIN_CACHE_TTL).await?;
        Ok(())
    }
}
