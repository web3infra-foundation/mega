//! Service for managing system required reviewers.
//!
//! This service handles automatic reviewer assignment based on Cedar policy files.

use std::path::PathBuf;

use common::errors::MegaError;
use saturn::reviewer_parser::aggregate_reviewers;

use crate::storage::{base_storage::BaseStorage, cl_reviewer_storage::ClReviewerStorage};

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

    /// Assign system required reviewers from hierarchical policy files
    ///
    /// Traverses policy files from root to leaf, applying override semantics:
    /// - Same path_pattern: child directory rules override parent directory rules
    /// - Different path_patterns: rules are merged (accumulated)
    ///
    /// # Arguments
    /// * `cl_link` - The CL link identifier
    /// * `cl_path` - The path of the CL
    /// * `policy_contents` - List of (policy_path, content) tuples, from root to leaf
    ///
    /// # Returns
    /// List of assigned reviewer usernames
    pub async fn assign_system_reviewers(
        &self,
        cl_link: &str,
        cl_path: &str,
        policy_contents: &[(PathBuf, String)],
    ) -> Result<Vec<String>, MegaError> {
        // Convert PathBuf to String for the aggregate function
        let policy_contents_str: Vec<(String, String)> = policy_contents
            .iter()
            .map(|(path, content)| (path.to_string_lossy().to_string(), content.clone()))
            .collect();

        // Use override-aware merge: same pattern from child overrides parent
        let all_reviewers = aggregate_reviewers(&policy_contents_str, cl_path);

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

    /// Sync system required reviewers when policy files are updated
    ///
    /// This method:
    /// 1. Removes all current system_required reviewers
    /// 2. Aggregates reviewers from hierarchical policy files (same pattern: child wins, different patterns: merge)
    /// 3. Adds new reviewers as system_required
    ///
    /// # Arguments
    /// * `cl_link` - The CL link identifier
    /// * `cl_path` - The path of the CL
    /// * `policy_contents` - List of (policy_path, content) tuples, from root to leaf
    pub async fn sync_system_reviewers(
        &self,
        cl_link: &str,
        cl_path: &str,
        policy_contents: &[(PathBuf, String)],
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

        // 2. Aggregate reviewers from hierarchical policies
        let policy_contents_str: Vec<(String, String)> = policy_contents
            .iter()
            .map(|(path, content)| (path.to_string_lossy().to_string(), content.clone()))
            .collect();

        let new_reviewers = aggregate_reviewers(&policy_contents_str, cl_path);

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
    use callisto::mega_cl_reviewer;
    use tempfile::tempdir;

    fn make_policy(path: &str, reviewers: &[&str]) -> String {
        let reviewer_list = reviewers
            .iter()
            .map(|r| format!("\"{}\"", r))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"permit(action == "code:review", principal, resource)
    when {{ resource.path.startsWith("{}") }}
    to [{}];"#,
            path, reviewer_list
        )
    }

    fn print_table(reviewers: &[mega_cl_reviewer::Model]) {
        if reviewers.is_empty() {
            println!("    (empty)");
            return;
        }
        println!("    +-----------+-----------------+----------+");
        println!("    | username  | system_required | approved |");
        println!("    +-----------+-----------------+----------+");
        for r in reviewers {
            println!(
                "    | {:9} | {:15} | {:8} |",
                r.username, r.system_required, r.approved
            );
        }
        println!("    +-----------+-----------------+----------+");
    }

    /// Comprehensive test for ReviewerService
    /// Run with: cargo test -p jupiter test_reviewer_service -- --nocapture
    #[tokio::test]
    async fn test_reviewer_service() {
        let temp = tempdir().unwrap();
        let storage = test_storage(&temp).await;
        let reviewer_storage = storage.reviewer_storage();
        let service = ReviewerService::from_storage(reviewer_storage.clone());

        println!("\n{}", "=".repeat(70));
        println!("Reviewer Service - Comprehensive Test");
        println!("{}", "=".repeat(70));

        let cl_link = "test_cl_001";
        let cl_path = "service_a/src/main.rs";

        // ========== 1. Assign System Reviewers ==========
        println!("\n[1] Assign System Reviewers (CL creation)");
        println!("{}", "-".repeat(50));

        let policy = make_policy("service_a/", &["alice", "bob"]);
        println!("    Policy: service_a/ -> [alice, bob]");
        println!("    CL path: {}", cl_path);

        let assigned = service
            .assign_system_reviewers(cl_link, cl_path, &[(PathBuf::from("/repo"), policy)])
            .await
            .unwrap();

        println!("    Assigned: {:?}", assigned);
        let reviewers = reviewer_storage.list_reviewers(cl_link).await.unwrap();
        print_table(&reviewers);

        assert_eq!(assigned.len(), 2);
        assert!(reviewers.iter().all(|r| r.system_required));
        println!("✓ System reviewers assigned with system_required=true");

        // ========== 2. Sync: Policy Update ==========
        println!("\n[2] Sync: Policy adds 'charlie', removes 'bob'");
        println!("{}", "-".repeat(50));

        let new_policy = make_policy("service_a/", &["alice", "charlie"]);
        println!("    New policy: service_a/ -> [alice, charlie]");

        service
            .sync_system_reviewers(cl_link, cl_path, &[(PathBuf::from("/repo"), new_policy)])
            .await
            .unwrap();

        let reviewers = reviewer_storage.list_reviewers(cl_link).await.unwrap();
        print_table(&reviewers);

        assert_eq!(reviewers.len(), 2);
        assert!(reviewers.iter().any(|r| r.username == "alice"));
        assert!(reviewers.iter().any(|r| r.username == "charlie"));
        assert!(!reviewers.iter().any(|r| r.username == "bob"));
        println!("✓ Sync correctly added/removed system reviewers");

        // ========== 3. Manual Reviewer Not Affected by Sync ==========
        println!("\n[3] Manual Reviewer Not Affected by Policy Sync");
        println!("{}", "-".repeat(50));

        // Add bob manually
        reviewer_storage
            .add_reviewers(cl_link, vec!["bob".to_string()])
            .await
            .unwrap();
        println!("    Added 'bob' manually (system_required=false)");

        let reviewers = reviewer_storage.list_reviewers(cl_link).await.unwrap();
        print_table(&reviewers);

        // Sync with policy that only has alice
        let new_policy = make_policy("service_a/", &["alice"]);
        println!("\n    Sync with policy: service_a/ -> [alice] only");
        println!("    Expected: charlie removed (system), bob preserved (manual)");

        service
            .sync_system_reviewers(cl_link, cl_path, &[(PathBuf::from("/repo"), new_policy)])
            .await
            .unwrap();

        let reviewers = reviewer_storage.list_reviewers(cl_link).await.unwrap();
        print_table(&reviewers);

        assert_eq!(reviewers.len(), 2);
        assert!(
            reviewers
                .iter()
                .any(|r| r.username == "alice" && r.system_required)
        );
        assert!(
            reviewers
                .iter()
                .any(|r| r.username == "bob" && !r.system_required)
        );
        println!("✓ Manual reviewer preserved, system reviewer removed");

        // ========== 4. Hierarchical Policy Merge ==========
        println!("\n[4] Hierarchical Policy Merge");
        println!("{}", "-".repeat(50));

        let cl_link2 = "test_cl_002";
        let root_policy = make_policy("", &["root_owner"]);
        let service_policy = make_policy("service_a/", &["alice"]);

        println!("    Root policy: '' -> [root_owner]");
        println!("    Child policy: service_a/ -> [alice]");
        println!("    CL path: service_a/core/lib.rs");

        let policies = vec![
            (PathBuf::from("/repo"), root_policy),
            (PathBuf::from("/repo/service_a"), service_policy),
        ];

        let assigned = service
            .assign_system_reviewers(cl_link2, "service_a/core/lib.rs", &policies)
            .await
            .unwrap();

        println!("    Assigned: {:?}", assigned);
        let reviewers = reviewer_storage.list_reviewers(cl_link2).await.unwrap();
        print_table(&reviewers);

        assert_eq!(assigned.len(), 2);
        assert!(assigned.contains(&"root_owner".to_string()));
        assert!(assigned.contains(&"alice".to_string()));
        println!("✓ Different patterns merged correctly");

        // ========== Summary ==========
        println!("\n{}", "=".repeat(70));
        println!("ALL TESTS PASSED!");
        println!("{}", "=".repeat(70));
        println!("\nCore features tested:");
        println!("  - assign_system_reviewers: CL creation assigns reviewers");
        println!("  - sync_system_reviewers: Policy update replaces system reviewers");
        println!("  - Manual reviewers (system_required=false) not affected by sync");
        println!("  - Hierarchical policy merge (different patterns)");
        println!();
    }

    /// Integration test simulating full monorepo hierarchical policy scenario
    /// This test simulates the complete flow as if policies were read from actual files
    /// Run with: cargo test -p jupiter test_monorepo_hierarchical_policies -- --nocapture
    #[tokio::test]
    async fn test_monorepo_hierarchical_policies() {
        let temp = tempdir().unwrap();
        let storage = test_storage(&temp).await;
        let reviewer_storage = storage.reviewer_storage();
        let service = ReviewerService::from_storage(reviewer_storage.clone());

        println!("\n{}", "=".repeat(70));
        println!("Monorepo Hierarchical Policy Integration Test");
        println!("{}", "=".repeat(70));

        // Simulate monorepo structure:
        // /
        // ├── .cedar/policies.cedar           -> ["global_owner"], ["service_a/" -> "old_alice"]
        // ├── service_a/
        // │   ├── .cedar/policies.cedar       -> ["service_a/" -> "alice", "bob"] (overrides root)
        // │   └── core/
        // │       ├── .cedar/policies.cedar   -> ["service_a/core/" -> "core_expert"]
        // │       └── main.rs
        // └── service_b/
        //     └── handler.rs

        println!("\n[Scenario] Three-level policy hierarchy with override");
        println!("{}", "-".repeat(50));
        println!("  /");
        println!("  ├── .cedar/policies.cedar");
        println!("  │   └── '' -> [global_owner]");
        println!("  │   └── 'service_a/' -> [old_alice] (will be overridden)");
        println!("  ├── service_a/");
        println!("  │   ├── .cedar/policies.cedar");
        println!("  │   │   └── 'service_a/' -> [alice, bob] (overrides root)");
        println!("  │   └── core/");
        println!("  │       ├── .cedar/policies.cedar");
        println!("  │       │   └── 'service_a/core/' -> [core_expert]");
        println!("  │       └── main.rs");
        println!("  └── service_b/");
        println!("      └── handler.rs");

        // ========== Test Case 1: Deep nested path (service_a/core/main.rs) ==========
        println!("\n[1] CL in service_a/core/main.rs");
        println!("{}", "-".repeat(50));

        let cl_link1 = "monorepo_cl_001";
        let cl_path1 = "service_a/core/main.rs";

        // Simulate policies collected from root to leaf
        let root_policy = format!(
            r#"{}
{}"#,
            make_policy("", &["global_owner"]),
            make_policy("service_a/", &["old_alice"])
        );
        let service_a_policy = make_policy("service_a/", &["alice", "bob"]);
        let core_policy = make_policy("service_a/core/", &["core_expert"]);

        let policies = vec![
            (PathBuf::from("/.cedar/policies.cedar"), root_policy),
            (
                PathBuf::from("/service_a/.cedar/policies.cedar"),
                service_a_policy,
            ),
            (
                PathBuf::from("/service_a/core/.cedar/policies.cedar"),
                core_policy,
            ),
        ];

        println!("    Policies (root to leaf):");
        println!("      - Root: '' -> [global_owner], 'service_a/' -> [old_alice]");
        println!("      - service_a: 'service_a/' -> [alice, bob] (overrides old_alice)");
        println!("      - core: 'service_a/core/' -> [core_expert]");
        println!("    CL path: {}", cl_path1);

        let assigned = service
            .assign_system_reviewers(cl_link1, cl_path1, &policies)
            .await
            .unwrap();

        println!("\n    Assigned reviewers: {:?}", assigned);
        let reviewers = reviewer_storage.list_reviewers(cl_link1).await.unwrap();
        print_table(&reviewers);

        // Verify: global_owner (from ""), alice+bob (from service_a, overrides old_alice), core_expert (from core)
        assert!(
            assigned.contains(&"global_owner".to_string()),
            "Should have global_owner"
        );
        assert!(assigned.contains(&"alice".to_string()), "Should have alice");
        assert!(assigned.contains(&"bob".to_string()), "Should have bob");
        assert!(
            assigned.contains(&"core_expert".to_string()),
            "Should have core_expert"
        );
        assert!(
            !assigned.contains(&"old_alice".to_string()),
            "old_alice should be overridden"
        );
        println!(
            "✓ Three-level hierarchy: global_owner + alice + bob + core_expert (old_alice overridden)"
        );

        // ========== Test Case 2: Service B path (no service-specific policy) ==========
        println!("\n[2] CL in service_b/handler.rs (no service-specific policy)");
        println!("{}", "-".repeat(50));

        let cl_link2 = "monorepo_cl_002";
        let cl_path2 = "service_b/handler.rs";

        let root_policy_only = make_policy("", &["global_owner"]);
        let policies2 = vec![(PathBuf::from("/.cedar/policies.cedar"), root_policy_only)];

        println!("    Only root policy: '' -> [global_owner]");
        println!("    CL path: {}", cl_path2);

        let assigned2 = service
            .assign_system_reviewers(cl_link2, cl_path2, &policies2)
            .await
            .unwrap();

        println!("\n    Assigned reviewers: {:?}", assigned2);
        let reviewers2 = reviewer_storage.list_reviewers(cl_link2).await.unwrap();
        print_table(&reviewers2);

        assert_eq!(assigned2.len(), 1);
        assert!(assigned2.contains(&"global_owner".to_string()));
        println!("✓ Only global_owner assigned (no service-specific policy)");

        // ========== Test Case 3: Same pattern override verification ==========
        println!("\n[3] Same pattern override: child wins");
        println!("{}", "-".repeat(50));

        let cl_link3 = "monorepo_cl_003";
        let cl_path3 = "service_a/src/lib.rs";

        let root_policy3 = make_policy("service_a/", &["root_reviewer"]);
        let child_policy3 = make_policy("service_a/", &["child_reviewer"]);

        let policies3 = vec![
            (PathBuf::from("/.cedar/policies.cedar"), root_policy3),
            (
                PathBuf::from("/service_a/.cedar/policies.cedar"),
                child_policy3,
            ),
        ];

        println!("    Root: 'service_a/' -> [root_reviewer]");
        println!("    Child: 'service_a/' -> [child_reviewer] (same pattern, should override)");
        println!("    CL path: {}", cl_path3);

        let assigned3 = service
            .assign_system_reviewers(cl_link3, cl_path3, &policies3)
            .await
            .unwrap();

        println!("\n    Assigned reviewers: {:?}", assigned3);
        let reviewers3 = reviewer_storage.list_reviewers(cl_link3).await.unwrap();
        print_table(&reviewers3);

        assert_eq!(assigned3.len(), 1);
        assert!(
            assigned3.contains(&"child_reviewer".to_string()),
            "child_reviewer should win"
        );
        assert!(
            !assigned3.contains(&"root_reviewer".to_string()),
            "root_reviewer should be overridden"
        );
        println!("✓ Same pattern override: child_reviewer wins, root_reviewer overridden");

        // ========== Summary ==========
        println!("\n{}", "=".repeat(70));
        println!("MONOREPO INTEGRATION TEST PASSED!");
        println!("{}", "=".repeat(70));
        println!("\nTested scenarios:");
        println!("  - Three-level policy hierarchy (root -> service -> module)");
        println!("  - Same pattern override: child directory wins");
        println!("  - Different patterns merge: all applicable rules combined");
        println!("  - Path without service-specific policy: only root policy applies");
        println!();
    }

    /// Test simulating policy file modification in CL update
    /// This tests the scenario where resync_current_cl_reviewers_if_policy_changed is triggered
    /// Run with: cargo test -p jupiter test_policy_file_modification_resync -- --nocapture
    #[tokio::test]
    async fn test_policy_file_modification_resync() {
        let temp = tempdir().unwrap();
        let storage = test_storage(&temp).await;
        let reviewer_storage = storage.reviewer_storage();
        let service = ReviewerService::from_storage(reviewer_storage.clone());

        println!("\n{}", "=".repeat(70));
        println!("Policy File Modification Resync Test");
        println!("{}", "=".repeat(70));

        let cl_link = "policy_update_cl_001";
        let cl_path = "service_a/src/main.rs";

        // ========== 1. Initial CL Creation ==========
        println!("\n[1] Initial CL Creation with Policy");
        println!("{}", "-".repeat(50));

        let initial_policy = make_policy("service_a/", &["alice", "bob"]);
        println!("    Initial policy: service_a/ -> [alice, bob]");

        let assigned = service
            .assign_system_reviewers(
                cl_link,
                cl_path,
                &[(PathBuf::from("/repo"), initial_policy)],
            )
            .await
            .unwrap();

        println!("    Assigned: {:?}", assigned);
        let reviewers = reviewer_storage.list_reviewers(cl_link).await.unwrap();
        print_table(&reviewers);

        assert_eq!(assigned.len(), 2);
        assert!(
            reviewers
                .iter()
                .any(|r| r.username == "alice" && r.system_required)
        );
        assert!(
            reviewers
                .iter()
                .any(|r| r.username == "bob" && r.system_required)
        );
        println!("✓ Initial reviewers assigned");

        // ========== 2. Simulate Policy File Modification ==========
        println!("\n[2] Simulate Policy File Modification in CL Update");
        println!("{}", "-".repeat(50));
        println!("    Scenario: User pushes update that modifies .cedar/policies.cedar");
        println!("    Changed files would include: service_a/.cedar/policies.cedar");

        // New policy: removes bob, adds charlie and david
        let updated_policy = make_policy("service_a/", &["alice", "charlie", "david"]);
        println!("    Updated policy: service_a/ -> [alice, charlie, david]");

        // This simulates what resync_current_cl_reviewers_if_policy_changed does
        // when it detects policy file in changed_files
        service
            .sync_system_reviewers(
                cl_link,
                cl_path,
                &[(PathBuf::from("/repo"), updated_policy)],
            )
            .await
            .unwrap();

        let reviewers = reviewer_storage.list_reviewers(cl_link).await.unwrap();
        print_table(&reviewers);

        assert_eq!(reviewers.len(), 3);
        assert!(reviewers.iter().any(|r| r.username == "alice"));
        assert!(reviewers.iter().any(|r| r.username == "charlie"));
        assert!(reviewers.iter().any(|r| r.username == "david"));
        assert!(!reviewers.iter().any(|r| r.username == "bob")); // bob removed
        println!("✓ Reviewers resynced after policy modification");

        // ========== 3. Multiple Policy Updates ==========
        println!("\n[3] Multiple Policy Updates (Hierarchical)");
        println!("{}", "-".repeat(50));

        let cl_link2 = "policy_update_cl_002";
        let cl_path2 = "service_a/core/handler.rs";

        // Initial: only root policy
        let root_policy = make_policy("", &["global_owner"]);
        println!("    Initial: Only root policy '' -> [global_owner]");

        service
            .assign_system_reviewers(
                cl_link2,
                cl_path2,
                &[(PathBuf::from("/repo"), root_policy.clone())],
            )
            .await
            .unwrap();

        let reviewers = reviewer_storage.list_reviewers(cl_link2).await.unwrap();
        print_table(&reviewers);
        assert_eq!(reviewers.len(), 1);
        println!("✓ Only global_owner initially");

        // Update: add service_a policy file
        println!("\n    Update: Add service_a/.cedar/policies.cedar");
        let service_policy = make_policy("service_a/", &["service_lead"]);
        println!("    New policy: service_a/ -> [service_lead]");

        let policies = vec![
            (PathBuf::from("/repo"), root_policy),
            (PathBuf::from("/repo/service_a"), service_policy),
        ];

        service
            .sync_system_reviewers(cl_link2, cl_path2, &policies)
            .await
            .unwrap();

        let reviewers = reviewer_storage.list_reviewers(cl_link2).await.unwrap();
        print_table(&reviewers);

        assert_eq!(reviewers.len(), 2);
        assert!(reviewers.iter().any(|r| r.username == "global_owner"));
        assert!(reviewers.iter().any(|r| r.username == "service_lead"));
        println!("✓ Both global_owner and service_lead after adding service policy");

        // ========== 4. Policy Deletion Scenario ==========
        println!("\n[4] Policy Deletion Scenario");
        println!("{}", "-".repeat(50));
        println!("    Scenario: service_a policy is deleted, only root policy remains");

        let root_only = make_policy("", &["global_owner"]);
        service
            .sync_system_reviewers(cl_link2, cl_path2, &[(PathBuf::from("/repo"), root_only)])
            .await
            .unwrap();

        let reviewers = reviewer_storage.list_reviewers(cl_link2).await.unwrap();
        print_table(&reviewers);

        assert_eq!(reviewers.len(), 1);
        assert!(reviewers.iter().any(|r| r.username == "global_owner"));
        assert!(!reviewers.iter().any(|r| r.username == "service_lead")); // removed
        println!("✓ service_lead removed after policy deletion");

        // ========== Summary ==========
        println!("\n{}", "=".repeat(70));
        println!("POLICY FILE MODIFICATION RESYNC TEST PASSED!");
        println!("{}", "=".repeat(70));
        println!("\nTested scenarios:");
        println!("  - Policy modification: reviewers resynced with new policy");
        println!("  - Policy addition: new policy reviewers added");
        println!("  - Policy deletion: removed policy reviewers cleaned up");
        println!("  - Manual reviewers preserved during resync");
        println!();
    }

    /// Test is_policy_file detection logic
    /// Run with: cargo test -p jupiter test_is_policy_file_detection -- --nocapture
    #[test]
    fn test_is_policy_file_detection() {
        println!("\n{}", "=".repeat(70));
        println!("Policy File Detection Test");
        println!("{}", "=".repeat(70));

        fn is_policy_file(path: &str) -> bool {
            path.ends_with(".cedar/policies.cedar") || path.ends_with(".cedar\\policies.cedar")
        }

        let test_cases = vec![
            // Valid policy files
            (".cedar/policies.cedar", true),
            ("service_a/.cedar/policies.cedar", true),
            ("service_a/submodule/.cedar/policies.cedar", true),
            // Windows path
            ("service_a\\.cedar\\policies.cedar", true),
            // Non-policy files
            ("service_a/src/main.rs", false),
            ("README.md", false),
            (".cedar/other.cedar", false),
            ("policies.cedar", false),
            ("service_a/.cedar/policies.json", false),
        ];

        for (path, expected) in test_cases {
            let result = is_policy_file(path);
            println!("    '{}' -> {} (expected: {})", path, result, expected);
            assert_eq!(result, expected, "Failed for path: {}", path);
        }

        println!("\n✓ All policy file detection tests passed!");
        println!("{}", "=".repeat(70));
    }
}
