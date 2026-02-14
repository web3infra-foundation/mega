/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{ProjectRelativePath, TargetLabel};

/// The output of running `buck2 uquery --json owner(...)`.
/// Maps file paths to the targets that own them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owners(HashMap<ProjectRelativePath, Vec<TargetLabel>>);

impl Owners {
    /// Create a new Owners from a JSON string returned by Buck2
    pub fn from_json(json_str: &str) -> anyhow::Result<Self> {
        let raw_map: HashMap<String, Vec<String>> = serde_json::from_str(json_str)?;

        let owners_map = raw_map
            .into_iter()
            .map(|(path_str, target_strs)| {
                (
                    ProjectRelativePath::new(&path_str),
                    target_strs
                        .into_iter()
                        .map(|target_str| TargetLabel::new(&target_str))
                        .collect(),
                )
            })
            .collect();

        Ok(Self(owners_map))
    }

    /// Create a new empty Owners
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Create Owners from a HashMap
    pub fn from_map(map: HashMap<ProjectRelativePath, Vec<TargetLabel>>) -> Self {
        Self(map)
    }

    /// Get the owners for a specific file path
    pub fn get(&self, path: &ProjectRelativePath) -> Option<&Vec<TargetLabel>> {
        self.0.get(path)
    }

    /// Get all unique target labels across all files
    pub fn all_targets(&self) -> impl Iterator<Item = &TargetLabel> {
        self.0.values().flat_map(|targets| targets.iter())
    }
}

impl Default for Owners {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owners_from_json() {
        let json_str = r#"{
            "fbcode/target_determinator/td_util/src/buck/run.rs": [
                "fbcode//target_determinator/td_util:buck-unittest",
                "fbcode//target_determinator/td_util:buck"
            ],
            "fbcode/another/file.rs": [
                "fbcode//another:target"
            ]
        }"#;

        let owners = Owners::from_json(json_str).unwrap();

        let run_rs_path =
            ProjectRelativePath::new("fbcode/target_determinator/td_util/src/buck/run.rs");
        let targets = owners.get(&run_rs_path).unwrap();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&TargetLabel::new(
            "fbcode//target_determinator/td_util:buck-unittest"
        )));
        assert!(targets.contains(&TargetLabel::new(
            "fbcode//target_determinator/td_util:buck"
        )));

        let another_path = ProjectRelativePath::new("fbcode/another/file.rs");
        let another_targets = owners.get(&another_path).unwrap();
        assert_eq!(another_targets.len(), 1);
        assert!(another_targets.contains(&TargetLabel::new("fbcode//another:target")));

        // Test all_targets iterator
        let all_targets: Vec<_> = owners.all_targets().collect();
        assert_eq!(all_targets.len(), 3);
    }

    #[test]
    fn test_owners_from_map() {
        let mut map = HashMap::new();
        map.insert(
            ProjectRelativePath::new("test/file.rs"),
            vec![TargetLabel::new("test//target:name")],
        );

        let owners = Owners::from_map(map);

        let path = ProjectRelativePath::new("test/file.rs");
        let targets = owners.get(&path).unwrap();
        assert_eq!(targets.len(), 1);
        assert!(targets.contains(&TargetLabel::new("test//target:name")));
    }

    #[test]
    fn test_owners_new() {
        let owners = Owners::new();
        let empty_path = ProjectRelativePath::new("nonexistent.rs");
        assert!(owners.get(&empty_path).is_none());

        let all_targets: Vec<_> = owners.all_targets().collect();
        assert!(all_targets.is_empty());
    }
}
