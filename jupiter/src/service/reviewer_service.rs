//! Service for managing system required reviewers.
//!
//! This service handles automatic reviewer assignment based on Cedar policy files.

use std::collections::HashSet;
use std::path::PathBuf;

use common::errors::MegaError;
use saturn::reviewer_parser::aggregate_reviewers;

use crate::storage::{base_storage::BaseStorage, cl_reviewer_storage::ClReviewerStorage};

/// Convert a file path to its logical directory path for Cedar policy matching.
///
/// For policy files (e.g., "servicea/.cedar/policies.cedar"), returns the parent
/// directory path with trailing slash (e.g., "servicea/").
/// For regular files, returns the path unchanged.
fn to_policy_match_path(file_path: &str) -> String {
    if file_path.ends_with(".cedar/policies.cedar") || file_path.ends_with(".cedar\\policies.cedar")
    {
        let path = std::path::Path::new(file_path);
        let parent = path.parent().unwrap_or(std::path::Path::new(""));
        let logical_parent = parent.parent().unwrap_or(std::path::Path::new(""));

        let mut p = logical_parent.to_string_lossy().to_string();
        if !p.is_empty() && !p.ends_with('/') {
            p.push('/');
        }
        p
    } else {
        file_path.to_string()
    }
}

/// Aggregate reviewers from policy contents for all changed files.
fn collect_reviewers(
    policy_contents: &[(PathBuf, String)],
    changed_files: &[String],
) -> Vec<String> {
    let policy_contents_str: Vec<(String, String)> = policy_contents
        .iter()
        .map(|(path, content)| (path.to_string_lossy().to_string(), content.clone()))
        .collect();

    let mut all_reviewers_set: HashSet<String> = HashSet::new();

    for file_path in changed_files {
        let path_to_check = to_policy_match_path(file_path);
        let reviewers = aggregate_reviewers(&policy_contents_str, &path_to_check);
        for reviewer in reviewers {
            all_reviewers_set.insert(reviewer);
        }
    }

    all_reviewers_set.into_iter().collect()
}

#[derive(Clone)]
pub struct ReviewerService {
    pub reviewer_storage: ClReviewerStorage,
}

impl ReviewerService {
    pub fn new(base_storage: BaseStorage) -> Self {
        Self {
            reviewer_storage: ClReviewerStorage { base: base_storage },
        }
    }

    /// Create ReviewerService from ClReviewerStorage directly
    pub fn from_storage(reviewer_storage: ClReviewerStorage) -> Self {
        Self { reviewer_storage }
    }

    /// Assign system required reviewers for multiple changed files
    ///
    /// This method iterates through all changed files and aggregates reviewers
    /// from policy files that match each file path.
    ///
    /// # Arguments
    /// * `cl_link` - The CL link identifier
    /// * `cl_path` - The path of the CL (used for logging)
    /// * `policy_contents` - List of (policy_path, content) tuples, from root to leaf
    /// * `changed_files` - List of changed file paths relative to CL root
    ///
    /// # Returns
    /// List of assigned reviewer usernames
    pub async fn assign_system_reviewers(
        &self,
        cl_link: &str,
        _cl_path: &str,
        policy_contents: &[(PathBuf, String)],
        changed_files: &[String],
    ) -> Result<Vec<String>, MegaError> {
        let all_reviewers = collect_reviewers(policy_contents, changed_files);

        if all_reviewers.is_empty() {
            return Ok(vec![]);
        }

        // Get existing reviewers to avoid duplicates
        let existing_reviewers: Vec<String> = self
            .reviewer_storage
            .list_reviewers(cl_link)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.username)
            .collect();

        // Filter out already existing reviewers
        let new_reviewers: Vec<String> = all_reviewers
            .iter()
            .filter(|r| !existing_reviewers.contains(r))
            .cloned()
            .collect();

        if !new_reviewers.is_empty() {
            self.reviewer_storage
                .add_reviewers(cl_link, new_reviewers)
                .await?;
        }

        // Mark all as system required
        self.reviewer_storage
            .update_system_required_reviewers(cl_link, &all_reviewers, true)
            .await?;

        Ok(all_reviewers)
    }

    /// Sync system required reviewers for multiple changed files
    ///
    /// This method:
    /// 1. Removes all current system_required reviewers
    /// 2. Aggregates reviewers from hierarchical policy files for all changed files
    /// 3. Adds new reviewers as system_required
    ///
    /// # Arguments
    /// * `cl_link` - The CL link identifier
    /// * `cl_path` - The path of the CL (used for logging)
    /// * `policy_contents` - List of (policy_path, content) tuples, from root to leaf
    /// * `changed_files` - List of changed file paths relative to CL root
    pub async fn sync_system_reviewers(
        &self,
        cl_link: &str,
        _cl_path: &str,
        policy_contents: &[(PathBuf, String)],
        changed_files: &[String],
    ) -> Result<(), MegaError> {
        // 1. Get and remove all current system_required reviewers
        let current_system: Vec<String> = self
            .reviewer_storage
            .list_reviewers(cl_link)
            .await?
            .into_iter()
            .filter(|r| r.system_required)
            .map(|r| r.username)
            .collect();

        if !current_system.is_empty() {
            self.reviewer_storage
                .remove_system_reviewers(cl_link, &current_system)
                .await?;
        }

        // 2. Aggregate reviewers from hierarchical policies for all changed files
        let new_reviewers = collect_reviewers(policy_contents, changed_files);

        // 3. Add new system reviewers
        if !new_reviewers.is_empty() {
            self.reviewer_storage
                .add_reviewers(cl_link, new_reviewers.clone())
                .await?;
            self.reviewer_storage
                .update_system_required_reviewers(cl_link, &new_reviewers, true)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_storage;
    use tempfile::tempdir;

    // --- Helpers ---

    fn make_policy(path: &str, reviewers: &[&str]) -> String {
        let list = reviewers
            .iter()
            .map(|r| format!("\"{}\"", r))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"permit(action == "code:review", principal, resource) when {{ resource.path.startsWith("{}") }} to [{}];"#,
            path, list
        )
    }

    // --- Tests ---

    /// Pattern Merge: Different patterns from root and child policies are merged.
    ///
    /// Root Policy: "" -> ["benjamin_747"]
    /// Child Policy: "servicea/" -> ["1550220889"]
    ///
    /// Result: Both reviewers are assigned (different patterns = merge).
    #[tokio::test]
    async fn test_real_world_scenario_merge() {
        let temp = tempdir().unwrap();
        let service = ReviewerService::from_storage(test_storage(&temp).await.reviewer_storage());
        let cl_link = "cl_real_world_merge";

        let policies = vec![
            (
                PathBuf::from("/project/test_cedar_policy/.cedar/policies.cedar"),
                make_policy("", &["benjamin_747"]),
            ),
            (
                PathBuf::from("/project/test_cedar_policy/servicea/.cedar/policies.cedar"),
                make_policy("servicea/", &["1550220889"]),
            ),
        ];

        // Only policy files changed (no code files)
        let changed_files = vec![
            ".cedar/policies.cedar".to_string(),
            "servicea/.cedar/policies.cedar".to_string(),
        ];

        let assigned = service
            .assign_system_reviewers(
                cl_link,
                "/project/test_cedar_policy",
                &policies,
                &changed_files,
            )
            .await
            .unwrap();
        println!("Assigned Reviewers: {:?}", assigned);

        assert!(
            assigned.contains(&"1550220889".to_string()),
            "Service owner must be assigned"
        );
        assert!(
            assigned.contains(&"benjamin_747".to_string()),
            "Root owner must be assigned"
        );

        assert_eq!(
            assigned.len(),
            2,
            "Should have exactly 2 reviewers: Global Owner + Service Owner"
        );
    }

    /// Same Pattern Override: Child policy overrides parent for identical patterns.
    ///
    /// Root Policy: "servicea/" -> ["old_owner"]
    /// Child Policy: "servicea/" -> ["1550220889"]
    ///
    /// Result: Only child's reviewer is assigned (same pattern = override).
    #[tokio::test]
    async fn test_real_world_scenario_override() {
        let temp = tempdir().unwrap();
        let service = ReviewerService::from_storage(test_storage(&temp).await.reviewer_storage());
        let cl_link = "cl_real_world_override";

        let policies = vec![
            (
                PathBuf::from("/project/test_cedar_policy/.cedar/policies.cedar"),
                make_policy("servicea/", &["old_owner"]),
            ),
            (
                PathBuf::from("/project/test_cedar_policy/servicea/.cedar/policies.cedar"),
                make_policy("servicea/", &["1550220889"]),
            ),
        ];

        let changed_files = vec!["servicea/core/logic.rs".to_string()];
        let assigned = service
            .assign_system_reviewers(
                cl_link,
                "/project/test_cedar_policy",
                &policies,
                &changed_files,
            )
            .await
            .unwrap();
        println!("Assigned Reviewers (Override Case): {:?}", assigned);

        assert!(
            assigned.contains(&"1550220889".to_string()),
            "Child policy should be applied"
        );
        assert!(
            !assigned.contains(&"old_owner".to_string()),
            "Parent policy should be overridden"
        );
    }

    /// Merge + Override combined: Different patterns merge, same patterns override.
    ///
    /// Root Policy: "" -> ["benjamin_747"], "servicea/" -> ["1510220889"]
    /// Child Policy: "servicea/" -> ["1550220889"]
    ///
    /// Result: benjamin_747 (merged) + 1550220889 (override winner), 1510220889 removed.
    #[tokio::test]
    async fn test_comprehensive_merge_and_override() {
        let temp = tempdir().unwrap();
        let service = ReviewerService::from_storage(test_storage(&temp).await.reviewer_storage());
        let cl_link = "cl_comprehensive_hybrid";

        let root_policy_content = format!(
            "{}\n{}",
            make_policy("", &["benjamin_747"]),
            make_policy("servicea/", &["1510220889"])
        );
        let child_policy_content = make_policy("servicea/", &["1550220889"]);

        let policies = vec![
            (
                PathBuf::from("/project/test_cedar_policy/.cedar/policies.cedar"),
                root_policy_content,
            ),
            (
                PathBuf::from("/project/test_cedar_policy/servicea/.cedar/policies.cedar"),
                child_policy_content,
            ),
        ];

        let changed_files = vec![
            ".cedar/policies.cedar".to_string(),
            "servicea/.cedar/policies.cedar".to_string(),
        ];

        let assigned = service
            .assign_system_reviewers(
                cl_link,
                "/project/test_cedar_policy",
                &policies,
                &changed_files,
            )
            .await
            .unwrap();
        println!("Assigned Reviewers (Hybrid): {:?}", assigned);

        assert!(
            assigned.contains(&"benjamin_747".to_string()),
            "Global owner must be preserved"
        );
        assert!(
            assigned.contains(&"1550220889".to_string()),
            "Child definition must be applied"
        );
        assert!(
            !assigned.contains(&"1510220889".to_string()),
            "Parent definition must be overridden"
        );

        assert_eq!(
            assigned.len(),
            2,
            "Should contain exactly Global Owner + New Service Owner"
        );
    }
}
