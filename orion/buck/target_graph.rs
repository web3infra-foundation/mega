/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::hash::Hasher;
use std::str::FromStr;

use dashmap::DashMap;
use dashmap::DashSet;
use fnv::FnvHasher;
use serde::Deserialize;
use serde::Serialize;

use crate::types::Package;
use crate::types::PatternType;
use crate::types::TargetPattern;

macro_rules! impl_string_storage {
    ($id_type:ident, $store_method:ident, $get_string_method:ident, $len_method:ident, $iter_method:ident, $map_field:ident) => {
        pub fn $store_method(&self, s: &str) -> $id_type {
            let id = s.parse().unwrap();
            self.$map_field.insert(id, s.to_string());
            id
        }

        pub fn $get_string_method(&self, id: $id_type) -> Option<String> {
            self.$map_field.get(&id).map(|v| v.clone())
        }

        pub fn $len_method(&self) -> usize {
            self.$map_field.len()
        }

        pub fn $iter_method(&self) -> impl Iterator<Item = ($id_type, String)> + '_ {
            self.$map_field
                .iter()
                .map(|entry| (*entry.key(), entry.value().clone()))
        }
    };
}

macro_rules! impl_collection_storage {
    ($key_type:ident, $value_type:ident, $store_method:ident, $add_method:ident, $get_method:ident, $len_method:ident, $iter_method:ident, $map_field:ident) => {
        pub fn $store_method(&self, key: $key_type, values: Vec<$value_type>) {
            if !values.is_empty() {
                self.$map_field.insert(key, values);
            }
        }

        pub fn $add_method(&self, key: $key_type, value: $value_type) {
            self.$map_field.entry(key).or_default().push(value);
        }

        pub fn $get_method(&self, key: $key_type) -> Option<Vec<$value_type>> {
            self.$map_field.get(&key).map(|v| v.clone())
        }

        pub fn $len_method(&self) -> usize {
            self.$map_field.len()
        }

        pub fn $iter_method(&self) -> impl Iterator<Item = ($key_type, Vec<$value_type>)> + '_ {
            self.$map_field
                .iter()
                .map(|entry| (*entry.key(), entry.value().clone()))
        }
    };
}

macro_rules! define_id_type {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize
        )]
        pub struct $name(u64);

        impl $name {
            pub fn as_u64(&self) -> u64 {
                self.0
            }
        }

        impl FromStr for $name {
            type Err = std::convert::Infallible;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let mut hasher = FnvHasher::default();
                hasher.write(s.as_bytes());
                Ok(Self(hasher.finish()))
            }
        }
    };
}

define_id_type!(TargetId);
define_id_type!(RuleTypeId);
define_id_type!(OncallId);
define_id_type!(LabelId);
define_id_type!(GlobPatternId);
define_id_type!(FileId);
define_id_type!(PackageId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinimizedBuckTarget {
    pub rule_type: RuleTypeId,
    pub oncall: Option<OncallId>,
    pub labels: Vec<LabelId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetGraph {
    // We store BuckTargets as ids as a form of string interning
    // These maps are used to convert Ids back to strings
    target_id_to_label: DashMap<TargetId, String>,
    rule_type_id_to_string: DashMap<RuleTypeId, String>,
    oncall_id_to_string: DashMap<OncallId, String>,
    label_id_to_string: DashMap<LabelId, String>,
    minimized_targets: DashMap<TargetId, MinimizedBuckTarget>,
    glob_pattern_id_to_string: DashMap<GlobPatternId, String>,
    package_id_to_path: DashMap<PackageId, String>,
    file_id_to_path: DashMap<FileId, String>,

    // Bidirectional dependency tracking
    target_id_to_rdeps: DashMap<TargetId, Vec<TargetId>>,
    target_id_to_deps: DashMap<TargetId, Vec<TargetId>>,

    // File relationship tracking for BZL imports
    file_id_to_rdeps: DashMap<FileId, Vec<FileId>>,

    // Package error tracking
    package_id_to_errors: DashMap<PackageId, Vec<String>>,

    // CI pattern storage
    target_id_to_ci_srcs: DashMap<TargetId, Vec<GlobPatternId>>,
    target_id_to_ci_srcs_must_match: DashMap<TargetId, Vec<GlobPatternId>>,

    // CI deps patterns storage
    target_id_to_ci_deps_package_patterns: DashMap<TargetId, Vec<PackageId>>,
    target_id_to_ci_deps_recursive_patterns: DashMap<TargetId, Vec<PackageId>>,

    // Targets that have the uses_sudo label
    targets_with_sudo_label: DashSet<TargetId>,
}

impl TargetGraph {
    pub fn new() -> Self {
        Self {
            target_id_to_label: DashMap::new(),
            rule_type_id_to_string: DashMap::new(),
            oncall_id_to_string: DashMap::new(),
            label_id_to_string: DashMap::new(),
            minimized_targets: DashMap::new(),
            glob_pattern_id_to_string: DashMap::new(),
            target_id_to_rdeps: DashMap::new(),
            target_id_to_deps: DashMap::new(),
            file_id_to_path: DashMap::new(),
            file_id_to_rdeps: DashMap::new(),
            package_id_to_path: DashMap::new(),
            package_id_to_errors: DashMap::new(),
            target_id_to_ci_srcs: DashMap::new(),
            target_id_to_ci_srcs_must_match: DashMap::new(),
            target_id_to_ci_deps_package_patterns: DashMap::new(),
            target_id_to_ci_deps_recursive_patterns: DashMap::new(),
            targets_with_sudo_label: DashSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.targets_len()
    }

    pub fn is_empty(&self) -> bool {
        self.targets_len() == 0
    }

    impl_string_storage!(
        TargetId,
        store_target,
        get_target_label,
        targets_len,
        iter_targets,
        target_id_to_label
    );

    impl_string_storage!(
        RuleTypeId,
        store_rule_type,
        get_rule_type_string,
        rule_types_len,
        iter_rule_types,
        rule_type_id_to_string
    );

    impl_string_storage!(
        OncallId,
        store_oncall,
        get_oncall_string,
        oncalls_len,
        iter_oncalls,
        oncall_id_to_string
    );

    impl_string_storage!(
        LabelId,
        store_label,
        get_label_string,
        labels_len,
        iter_labels,
        label_id_to_string
    );

    impl_string_storage!(
        GlobPatternId,
        store_glob_pattern,
        get_glob_pattern_string,
        glob_patterns_len,
        iter_glob_patterns,
        glob_pattern_id_to_string
    );

    impl_string_storage!(
        FileId,
        store_file,
        get_file_path,
        files_len,
        iter_files,
        file_id_to_path
    );

    impl_string_storage!(
        PackageId,
        store_package,
        get_package_path,
        packages_len,
        iter_packages,
        package_id_to_path
    );

    impl_collection_storage!(
        TargetId,
        GlobPatternId,
        store_ci_srcs,
        add_ci_src,
        get_ci_srcs,
        ci_srcs_len,
        iter_ci_srcs,
        target_id_to_ci_srcs
    );
    impl_collection_storage!(
        TargetId,
        GlobPatternId,
        store_ci_srcs_must_match,
        add_ci_src_must_match,
        get_ci_srcs_must_match,
        ci_srcs_must_match_len,
        iter_ci_srcs_must_match,
        target_id_to_ci_srcs_must_match
    );

    impl_collection_storage!(
        PackageId,
        String,
        store_errors,
        add_error,
        get_errors,
        errors_len,
        iter_packages_with_errors,
        package_id_to_errors
    );

    impl_collection_storage!(
        TargetId,
        PackageId,
        store_ci_deps_package_patterns,
        add_ci_deps_package_pattern,
        get_ci_deps_package_patterns,
        ci_deps_package_patterns_len,
        iter_ci_deps_package_patterns,
        target_id_to_ci_deps_package_patterns
    );
    impl_collection_storage!(
        TargetId,
        PackageId,
        store_ci_deps_recursive_patterns,
        add_ci_deps_recursive_pattern,
        get_ci_deps_recursive_patterns,
        ci_deps_recursive_patterns_len,
        iter_ci_deps_recursive_patterns,
        target_id_to_ci_deps_recursive_patterns
    );

    // Bidirectional dependencies storage - always maintains both directions
    pub fn add_rdep(&self, target_id: TargetId, dependent_target: TargetId) {
        // Note: We intentionally don't check for duplicate existence for performance reasons.
        // Store reverse dependency: dependent_target depends on target_id
        self.target_id_to_rdeps
            .entry(target_id)
            .or_default()
            .push(dependent_target);

        // Also store forward dependency: dependent_target -> target_id
        self.target_id_to_deps
            .entry(dependent_target)
            .or_default()
            .push(target_id);
    }

    pub fn remove_rdep(&self, target_id: TargetId, dependent_target: TargetId) {
        // Remove from reverse dependencies
        if let Some(mut rdeps) = self.target_id_to_rdeps.get_mut(&target_id) {
            rdeps.retain(|&id| id != dependent_target);
            if rdeps.is_empty() {
                drop(rdeps);
                self.target_id_to_rdeps.remove(&target_id);
            }
        }

        // Remove from forward dependencies
        if let Some(mut deps) = self.target_id_to_deps.get_mut(&dependent_target) {
            deps.retain(|&id| id != target_id);
            if deps.is_empty() {
                drop(deps);
                self.target_id_to_deps.remove(&dependent_target);
            }
        }
    }

    pub fn get_rdeps(&self, target_id: TargetId) -> Option<Vec<TargetId>> {
        self.target_id_to_rdeps.get(&target_id).map(|v| v.clone())
    }

    pub fn get_deps(&self, target_id: TargetId) -> Option<Vec<TargetId>> {
        self.target_id_to_deps.get(&target_id).map(|v| v.clone())
    }

    /// Remove a target and all its associated data from the graph
    ///
    /// This includes:
    /// - Removing the target from all other targets' dependencies
    /// - Removing all dependencies of this target
    /// - Removing all CI pattern associations
    /// - Removing the target from the target map
    pub fn remove_target(&self, target_id: TargetId) {
        // Get all targets this target depends on
        if let Some(deps) = self.get_deps(target_id) {
            // For each dependency, remove target_id from their rdeps
            for dep_id in deps {
                if let Some(mut rdeps) = self.target_id_to_rdeps.get_mut(&dep_id) {
                    rdeps.retain(|&id| id != target_id);
                    // Remove the entry if empty
                    if rdeps.is_empty() {
                        drop(rdeps);
                        self.target_id_to_rdeps.remove(&dep_id);
                    }
                }
            }
        }

        // Get all targets that depend on this target
        if let Some(rdeps) = self.get_rdeps(target_id) {
            // For each dependent, remove target_id from their deps
            for rdep_id in rdeps {
                if let Some(mut deps) = self.target_id_to_deps.get_mut(&rdep_id) {
                    deps.retain(|&id| id != target_id);
                    // Remove the entry if empty
                    if deps.is_empty() {
                        drop(deps);
                        self.target_id_to_deps.remove(&rdep_id);
                    }
                }
            }
        }

        // Clear dependency relationships
        self.target_id_to_deps.remove(&target_id);
        self.target_id_to_rdeps.remove(&target_id);

        // Remove CI pattern associations
        self.target_id_to_ci_srcs.remove(&target_id);
        self.target_id_to_ci_srcs_must_match.remove(&target_id);
        self.target_id_to_ci_deps_package_patterns
            .remove(&target_id);
        self.target_id_to_ci_deps_recursive_patterns
            .remove(&target_id);

        // Remove from sudo label set
        self.targets_with_sudo_label.remove(&target_id);

        // Remove target information
        self.target_id_to_label.remove(&target_id);
        self.minimized_targets.remove(&target_id);
    }

    pub fn get_all_targets(&self) -> impl Iterator<Item = TargetId> + '_ {
        self.target_id_to_label.iter().map(|entry| *entry.key())
    }

    pub fn store_minimized_target(&self, target_id: TargetId, target: MinimizedBuckTarget) {
        self.minimized_targets.insert(target_id, target);
    }

    pub fn get_minimized_target(&self, id: TargetId) -> Option<MinimizedBuckTarget> {
        self.minimized_targets.get(&id).map(|entry| entry.clone())
    }

    pub fn mark_target_has_sudo_label(&self, target_id: TargetId) {
        self.targets_with_sudo_label.insert(target_id);
    }

    pub fn has_sudo_label(&self, target_id: TargetId) -> bool {
        self.targets_with_sudo_label.contains(&target_id)
    }

    pub fn iter_targets_with_sudo_label(&self) -> impl Iterator<Item = TargetId> + '_ {
        self.targets_with_sudo_label.iter().map(|entry| *entry)
    }

    pub fn targets_with_sudo_label_len(&self) -> usize {
        self.targets_with_sudo_label.len()
    }

    // Size analysis methods
    pub fn rdeps_len(&self) -> usize {
        self.target_id_to_rdeps.len()
    }

    pub fn deps_len(&self) -> usize {
        self.target_id_to_deps.len()
    }

    // File reverse dependencies storage - similar to target rdeps
    pub fn add_file_rdep(&self, file_id: FileId, dependent_file: FileId) {
        // Note: We intentionally don't check for duplicate existence for performance reasons.
        self.file_id_to_rdeps
            .entry(file_id)
            .or_default()
            .push(dependent_file);
    }

    pub fn get_file_rdeps(&self, file_id: FileId) -> Option<Vec<FileId>> {
        self.file_id_to_rdeps.get(&file_id).map(|v| v.clone())
    }

    pub fn file_rdeps_len(&self) -> usize {
        self.file_id_to_rdeps.len()
    }

    pub fn minimized_targets_len(&self) -> usize {
        self.minimized_targets.len()
    }

    /// Display analysis of internal data structures
    pub fn print_size_analysis(&self) {
        // Create a vector of tuples (name, size) for all storage collections
        let sizes = vec![
            ("targets", self.targets_len()),
            ("rdeps", self.rdeps_len()),
            ("deps", self.deps_len()),
            ("rule_types", self.rule_types_len()),
            ("oncalls", self.oncalls_len()),
            ("labels", self.labels_len()),
            ("minimized_targets", self.minimized_targets_len()),
            ("glob_patterns", self.glob_patterns_len()),
            ("files", self.files_len()),
            ("file_rdeps", self.file_rdeps_len()),
            ("packages", self.packages_len()),
            ("errors", self.errors_len()),
            ("ci_srcs", self.ci_srcs_len()),
            ("ci_srcs_must_match", self.ci_srcs_must_match_len()),
            (
                "ci_deps_package_patterns",
                self.ci_deps_package_patterns_len(),
            ),
            (
                "ci_deps_recursive_patterns",
                self.ci_deps_recursive_patterns_len(),
            ),
            (
                "targets_with_sudo_label",
                self.targets_with_sudo_label_len(),
            ),
        ];

        tracing::info!("TargetGraph DashMap sizes:");
        for (name, size) in sizes {
            tracing::info!("  {}: {}", name, size);
        }
    }

    pub fn package_id_to_target_pattern(
        &self,
        package_id: PackageId,
        pattern_type: PatternType,
    ) -> Option<TargetPattern> {
        self.get_package_path(package_id)
            .map(|package_path| Package::new(&package_path).to_target_pattern(pattern_type))
    }
}

impl Default for TargetGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_id_creation() {
        let target_label = "fbcode//buck2:buck2";
        let id1 = target_label.parse::<TargetId>().unwrap();
        let id2 = target_label.parse::<TargetId>().unwrap();

        // Same string should produce same TargetId
        assert_eq!(id1, id2);

        // Different strings should produce different TargetIds
        let id3 = "fbcode//other:target".parse::<TargetId>().unwrap();
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_target_graph_basic_operations() {
        let graph = TargetGraph::new();

        let target1 = "fbcode//a:target1";
        let target2 = "fbcode//b:target2";

        let id1 = graph.store_target(target1);
        let id2 = graph.store_target(target2);

        assert_ne!(id1, id2);
        assert_eq!(graph.len(), 2);

        let id1_again = graph.store_target(target1);
        assert_eq!(id1, id1_again);
    }

    #[test]
    fn test_reverse_dependencies() {
        let graph = TargetGraph::new();

        let target1 = "fbcode//a:target1";
        let target2 = "fbcode//b:target2";
        let target3 = "fbcode//c:target3";

        let id1 = graph.store_target(target1);
        let id2 = graph.store_target(target2);
        let id3 = graph.store_target(target3);

        graph.add_rdep(id1, id2);
        graph.add_rdep(id1, id3);

        let rdeps = graph.get_rdeps(id1).unwrap();
        assert_eq!(rdeps.len(), 2);
        assert!(rdeps.contains(&id2));
        assert!(rdeps.contains(&id3));

        assert!(graph.get_rdeps(id2).is_none());
        assert!(graph.get_rdeps(id3).is_none());
    }

    #[test]
    fn test_serialization() {
        let graph = TargetGraph::new();

        let target1 = "fbcode//a:target1";
        let target2 = "fbcode//b:target2";
        let target3 = "fbcode//c:target3";

        let id1 = graph.store_target(target1);
        let id2 = graph.store_target(target2);
        let id3 = graph.store_target(target3);

        graph.add_rdep(id1, id2);
        graph.add_rdep(id1, id3);

        let json = serde_json::to_string(&graph).expect("Failed to serialize");
        let restored_graph: TargetGraph =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored_graph.len(), 3);

        let restored_rdeps1 = restored_graph.get_rdeps(id1).unwrap();
        assert_eq!(restored_rdeps1.len(), 2);
        assert!(restored_rdeps1.contains(&id2));
        assert!(restored_rdeps1.contains(&id3));

        assert_eq!(restored_graph.store_target(target1), id1);
        assert_eq!(restored_graph.store_target(target2), id2);
        assert_eq!(restored_graph.store_target(target3), id3);
    }

    #[test]
    fn test_new_id_types() {
        // Test TargetId
        let target1 = "fbcode//a:target1";
        let target2 = "fbcode//b:target2";
        let target_id1: TargetId = target1.parse().unwrap();
        let target_id2: TargetId = target2.parse().unwrap();
        assert_ne!(target_id1, target_id2);
        assert_eq!(target1.parse::<TargetId>().unwrap(), target_id1);

        // Test RuleTypeId
        let rule1 = "cpp_library";
        let rule2 = "python_library";
        let rule_id1: RuleTypeId = rule1.parse().unwrap();
        let rule_id2: RuleTypeId = rule2.parse().unwrap();
        assert_ne!(rule_id1, rule_id2);
        assert_eq!(rule1.parse::<RuleTypeId>().unwrap(), rule_id1);

        // Test OncallId
        let oncall1 = "team_a";
        let oncall2 = "team_b";
        let oncall_id1: OncallId = oncall1.parse().unwrap();
        let oncall_id2: OncallId = oncall2.parse().unwrap();
        assert_ne!(oncall_id1, oncall_id2);
        assert_eq!(oncall1.parse::<OncallId>().unwrap(), oncall_id1);

        // Test LabelId
        let label1 = "ci_test";
        let label2 = "production";
        let label_id1: LabelId = label1.parse().unwrap();
        let label_id2: LabelId = label2.parse().unwrap();
        assert_ne!(label_id1, label_id2);
        assert_eq!(label1.parse::<LabelId>().unwrap(), label_id1);
    }

    #[test]
    fn test_string_storage_and_retrieval() {
        let graph = TargetGraph::new();

        // Store and retrieve target
        let target_label = "fbcode//test:target";
        let target_id = graph.store_target(target_label);
        assert_eq!(
            graph.get_target_label(target_id),
            Some(target_label.to_string())
        );

        // Store and retrieve rule type
        let rule_type = "cpp_library";
        let rule_id = graph.store_rule_type(rule_type);
        assert_eq!(
            graph.get_rule_type_string(rule_id),
            Some(rule_type.to_string())
        );

        // Store and retrieve oncall
        let oncall = "team_efficiency";
        let oncall_id = graph.store_oncall(oncall);
        assert_eq!(graph.get_oncall_string(oncall_id), Some(oncall.to_string()));

        // Store and retrieve label
        let label = "ci_test";
        let label_id = graph.store_label(label);
        assert_eq!(graph.get_label_string(label_id), Some(label.to_string()));
    }

    #[test]
    fn test_minimized_target() {
        let graph = TargetGraph::new();

        let target_label = "fbcode//test:target";
        let target_id = graph.store_target(target_label);
        let rule_type_id = graph.store_rule_type("cpp_library");
        let oncall_id = graph.store_oncall("team_test");
        let label_id1 = graph.store_label("ci_test");
        let label_id2 = graph.store_label("production");

        let minimized = MinimizedBuckTarget {
            rule_type: rule_type_id,
            oncall: Some(oncall_id),
            labels: vec![label_id1, label_id2],
        };

        graph.store_minimized_target(target_id, minimized.clone());
        let retrieved = graph.get_minimized_target(target_id);
        assert_eq!(retrieved, Some(minimized));

        let non_existent_target_id: TargetId = "fbcode//non_existent:target".parse().unwrap();
        assert_eq!(graph.get_minimized_target(non_existent_target_id), None);
    }

    #[test]
    fn test_new_extended_id_types() {
        // Test GlobPatternId
        let pattern1 = "**/*.rs";
        let pattern2 = "**/*.py";
        let pattern_id1: GlobPatternId = pattern1.parse().unwrap();
        let pattern_id2: GlobPatternId = pattern2.parse().unwrap();
        assert_ne!(pattern_id1, pattern_id2);
        assert_eq!(pattern1.parse::<GlobPatternId>().unwrap(), pattern_id1);

        // Test FileId
        let file1 = "src/main.rs";
        let file2 = "src/lib.rs";
        let file_id1: FileId = file1.parse().unwrap();
        let file_id2: FileId = file2.parse().unwrap();
        assert_ne!(file_id1, file_id2);
        assert_eq!(file1.parse::<FileId>().unwrap(), file_id1);

        // Test PackageId
        let package1 = "fbcode//target_determinator";
        let package2 = "fbcode//target_determinator/btd";
        let package_id1: PackageId = package1.parse().unwrap();
        let package_id2: PackageId = package2.parse().unwrap();
        assert_ne!(package_id1, package_id2);
        assert_eq!(package1.parse::<PackageId>().unwrap(), package_id1);
    }

    #[test]
    fn test_remove_rdep_cleans_empty_entries() {
        let graph = TargetGraph::new();

        let target1 = "fbcode//a:target1";
        let target2 = "fbcode//b:target2";
        let target3 = "fbcode//c:target3";

        let id1 = graph.store_target(target1);
        let id2 = graph.store_target(target2);
        let id3 = graph.store_target(target3);

        // Add dependencies: target1 <- target2, target1 <- target3
        graph.add_rdep(id1, id2);
        graph.add_rdep(id1, id3);

        // Verify initial state
        assert_eq!(graph.rdeps_len(), 1);
        assert_eq!(graph.deps_len(), 2);
        assert_eq!(graph.get_rdeps(id1).unwrap().len(), 2);

        // Remove one dependency
        graph.remove_rdep(id1, id2);

        // Should still have entries as id1 still has rdeps
        assert_eq!(graph.rdeps_len(), 1);
        assert_eq!(graph.deps_len(), 1);
        assert_eq!(graph.get_rdeps(id1).unwrap().len(), 1);

        // Remove the last dependency
        graph.remove_rdep(id1, id3);

        // Should have removed the empty entries
        assert_eq!(graph.rdeps_len(), 0);
        assert_eq!(graph.deps_len(), 0);
        assert!(graph.get_rdeps(id1).is_none());
    }

    #[test]
    fn test_remove_target_removes_all_data() {
        let graph = TargetGraph::new();

        let target1 = "fbcode//a:target1";
        let target2 = "fbcode//b:target2";

        let id1 = graph.store_target(target1);
        let id2 = graph.store_target(target2);

        // Add a dependency and some metadata
        graph.add_rdep(id1, id2);

        let rule_type_id = graph.store_rule_type("cpp_library");
        let minimized = MinimizedBuckTarget {
            rule_type: rule_type_id,
            oncall: None,
            labels: vec![],
        };
        graph.store_minimized_target(id1, minimized);

        // Verify initial state
        assert_eq!(graph.len(), 2);
        assert_eq!(graph.rdeps_len(), 1);
        assert_eq!(graph.deps_len(), 1);
        assert!(graph.get_minimized_target(id1).is_some());

        // Remove target1
        graph.remove_target(id1);

        // Should have removed target from label map and minimized targets
        assert_eq!(graph.len(), 1);
        assert!(graph.get_minimized_target(id1).is_none());

        // Should have cleaned up empty dependency entries
        assert_eq!(graph.rdeps_len(), 0);
        assert_eq!(graph.deps_len(), 0);
        assert!(graph.get_rdeps(id1).is_none());
        assert!(graph.get_deps(id2).is_none());
    }
}
