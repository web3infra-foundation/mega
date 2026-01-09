//! Admin resolution from Cedar entity store.
//!
//! This module provides utilities for determining admin status from Cedar entity data.
//!
//! # Assumptions
//! - Only checks direct membership in `UserGroup::"admin"`
//! - Admin is the most privileged group; no groups inherit from it
//! - Path is fixed to `/project/.mega_cedar.json` (monorepo only)

use std::collections::HashSet;

use crate::entitystore::EntityStore;

/// Resolver for admin permissions based on Cedar entity data.
///
/// Provides O(1) lookup for admin status after construction from [`EntityStore`].
#[derive(Debug, Clone, Default)]
pub struct AdminResolver {
    admin_set: HashSet<String>,
}

impl AdminResolver {
    /// Create from [`EntityStore`] by extracting users in the admin group.
    ///
    /// Uses [`EntityStore::extract_admin_usernames`] which checks direct
    /// membership only (no BFS/DFS for group hierarchy).
    pub fn from_entity_store(store: &EntityStore) -> Self {
        let admin_set = store.extract_admin_usernames();
        Self { admin_set }
    }

    /// Check if the given username is an admin.
    ///
    /// This is an O(1) lookup against the pre-computed admin set.
    pub fn is_admin(&self, username: &str) -> bool {
        self.admin_set.contains(username)
    }

    /// Get a list of all admin usernames.
    ///
    /// Returns pure usernames (e.g., `"alice"`), not Cedar EUIDs.
    pub fn admin_list(&self) -> Vec<String> {
        let mut admins: Vec<String> = self.admin_set.iter().cloned().collect();
        admins.sort();
        admins
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitystore::generate_entity;

    #[test]
    fn test_admin_resolver_identifies_admin() {
        let admins = vec!["alice".to_string()];
        let entity_json = generate_entity(&admins, "/project").unwrap();
        let store: EntityStore = serde_json::from_str(&entity_json).unwrap();

        let resolver = AdminResolver::from_entity_store(&store);

        assert!(resolver.is_admin("alice"), "alice should be admin");
        assert!(!resolver.is_admin("bob"), "bob should not be admin");
    }

    #[test]
    fn test_admin_list_returns_all_admins() {
        let admins = vec!["genedna".to_string()];
        let entity_json = generate_entity(&admins, "/project").unwrap();
        let store: EntityStore = serde_json::from_str(&entity_json).unwrap();

        let resolver = AdminResolver::from_entity_store(&store);
        let admin_list = resolver.admin_list();

        assert_eq!(admin_list.len(), 1);
        assert!(admin_list.contains(&"genedna".to_string()));
    }
}
