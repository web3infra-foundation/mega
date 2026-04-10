/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::HashSet;

use api_model::buck2::{status::Status, types::ProjectRelativePath};
use td_util_buck::{
    cells::CellInfo,
    types::{CellPath, Package},
};

type ResolvedChange = Status<(CellPath, ProjectRelativePath)>;
type UnresolvedChange = (ProjectRelativePath, anyhow::Error);

#[derive(Default, Debug)]
pub struct Changes {
    paths: Vec<ResolvedChange>,
    cell_paths_set: HashSet<CellPath>,
}

impl Changes {
    pub fn new(
        cells: &CellInfo,
        changes: Vec<Status<ProjectRelativePath>>,
    ) -> anyhow::Result<Self> {
        let (paths, unresolved_paths) =
            map_changes_with_resolver(changes, |path| cells.unresolve(path));

        if !unresolved_paths.is_empty() {
            for (path, err) in unresolved_paths.iter().take(10) {
                tracing::warn!(
                    path = %path,
                    error = %err,
                    "Skipping change path that could not be mapped to a Buck cell."
                );
            }
            if unresolved_paths.len() > 10 {
                tracing::warn!(
                    unresolved_count = unresolved_paths.len(),
                    "Skipped additional unmapped change paths (showing first 10)."
                );
            }
        }

        Ok(Self::from_paths(paths))
    }

    fn from_paths(paths: Vec<ResolvedChange>) -> Self {
        let cell_paths_set = paths.iter().map(|x| x.get().0.clone()).collect();
        Self {
            paths,
            cell_paths_set,
        }
    }

    #[cfg(test)]
    pub fn testing(changes: &[Status<CellPath>]) -> Self {
        fn mk_project_path(path: &CellPath) -> ProjectRelativePath {
            ProjectRelativePath::new(path.path().as_str())
        }

        let paths: Vec<ResolvedChange> = changes
            .iter()
            .map(|status| status.map(|cell_path| (cell_path.clone(), mk_project_path(cell_path))))
            .collect();
        Self::from_paths(paths)
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn status_cell_paths(&self) -> impl Iterator<Item = Status<&CellPath>> {
        self.paths.iter().map(|x| x.map(|x| &x.0))
    }

    pub fn cell_paths(&self) -> impl Iterator<Item = &CellPath> {
        self.paths.iter().map(|x| &x.get().0)
    }

    pub fn project_paths(&self) -> impl Iterator<Item = &ProjectRelativePath> {
        self.paths.iter().map(|x| &x.get().1)
    }

    pub fn contains_cell_path(&self, path: &CellPath) -> bool {
        self.cell_paths_set.contains(path)
    }

    pub fn contains_package(&self, package: &Package) -> bool {
        // Check if the package directory itself is in changes
        if self.contains_cell_path(&package.as_cell_path()) {
            return true;
        }

        // Check if any BUCK file in this package is in changes
        // BUCK files define targets, so changes to them affect the package
        let package_str = package.as_str();
        for path in self.cell_paths() {
            let path_str = path.as_str();

            // Check if this path is a BUCK file in the package directory
            if let Some(relative) = path_str.strip_prefix(package_str) {
                // Handle both "cell//dir" and "cell//dir/" formats
                // strip_prefix("cell//dir") on "cell//dir/BUCK" yields "/BUCK"
                // strip_prefix("cell//dir/") on "cell//dir/BUCK" yields "BUCK"
                let relative = relative.strip_prefix('/').unwrap_or(relative);

                // Match BUCK or BUCK.v2 files directly in this package (not subdirectories)
                if relative == "BUCK" || relative == "BUCK.v2" || relative.starts_with("BUCK.") {
                    return true;
                }
            }
        }

        false
    }

    pub fn filter_by_cell_path(&self, f: impl Fn(&CellPath) -> bool) -> Changes {
        let paths = self
            .paths
            .iter()
            .filter(|x| f(&x.get().0))
            .cloned()
            .collect();
        Self::from_paths(paths)
    }

    pub fn filter_by_extension(&self, f: impl Fn(Option<&str>) -> bool) -> Changes {
        self.filter_by_cell_path(|x| f(x.extension()))
    }
}

fn map_changes_with_resolver(
    changes: Vec<Status<ProjectRelativePath>>,
    mut resolver: impl FnMut(&ProjectRelativePath) -> anyhow::Result<CellPath>,
) -> (Vec<ResolvedChange>, Vec<UnresolvedChange>) {
    let mut mapped = Vec::new();
    let mut unresolved = Vec::new();

    for change in changes {
        let project_path = change.get().clone();
        match resolver(&project_path) {
            Ok(cell_path) => {
                mapped.push(change.into_map(|path| (cell_path, path)));
            }
            Err(err) => unresolved.push((project_path, err)),
        }
    }

    (mapped, unresolved)
}

#[cfg(test)]
mod tests {
    use api_model::buck2::types::ProjectRelativePath;
    use td_util_buck::types::CellPath;

    use super::*;

    #[test]
    fn test_changes_empty() {
        let changes = Changes::default();
        assert!(changes.is_empty());
    }

    #[test]
    fn test_changes_new() {
        let cell_info = CellInfo::testing();
        let project_paths = vec![
            Status::Modified(ProjectRelativePath::new("src/lib.rs")),
            Status::Modified(ProjectRelativePath::new("src/main.rs")),
        ];
        let changes = Changes::new(&cell_info, project_paths).unwrap();
        assert!(!changes.is_empty());
        assert_eq!(changes.paths.len(), 2);
    }

    #[test]
    fn test_contains_cell_path() {
        let cell_path = CellPath::new("cell1//");
        let project_path = ProjectRelativePath::new("src/lib.rs");
        let paths = vec![Status::Added((cell_path.clone(), project_path))];
        let changes = Changes::from_paths(paths);
        assert!(changes.contains_cell_path(&cell_path));
    }

    #[test]
    fn test_filter_by_cell_path() {
        let cell_path1 = CellPath::new("cell1//");
        let cell_path2 = CellPath::new("cell2//");
        let project_path1 = ProjectRelativePath::new("src/lib.rs");
        let project_path2 = ProjectRelativePath::new("src/main.rs");
        let paths = vec![
            Status::Added((cell_path1.clone(), project_path1)),
            Status::Added((cell_path2.clone(), project_path2)),
        ];
        let changes = Changes::from_paths(paths);
        let filtered_changes = changes.filter_by_cell_path(|path| path == &cell_path1);
        assert_eq!(filtered_changes.paths.len(), 1);
        assert!(filtered_changes.contains_cell_path(&cell_path1));
        assert!(!filtered_changes.contains_cell_path(&cell_path2));
    }

    #[test]
    fn test_filter_by_extension() {
        let cell_path1 = CellPath::new("Cell1//foo/bar/cell1.rs");
        let cell_path2 = CellPath::new("Cell2//foo/baz/cell2.txt");
        let project_path1 = ProjectRelativePath::new("foo/bar/cell1.rs");
        let project_path2 = ProjectRelativePath::new("foo/baz/cell2.txt");
        let paths = vec![
            Status::Added((cell_path1.clone(), project_path1)),
            Status::Added((cell_path2.clone(), project_path2)),
        ];
        let changes = Changes::from_paths(paths);
        let filtered_changes = changes.filter_by_extension(|ext| ext == Some("rs"));
        assert_eq!(filtered_changes.paths.len(), 1);
        assert!(filtered_changes.contains_cell_path(&cell_path1));
        assert!(!filtered_changes.contains_cell_path(&cell_path2));
    }

    #[test]
    fn test_contains_package_with_buck_file() {
        // Test that modifying a BUCK file is detected as a package change
        let buck_path = CellPath::new("root//BUCK");
        let project_path = ProjectRelativePath::new("BUCK");
        let paths = vec![Status::Modified((buck_path, project_path))];
        let changes = Changes::from_paths(paths);

        let package = Package::new("root//");
        assert!(
            changes.contains_package(&package),
            "BUCK file change should be detected as package change"
        );
    }

    #[test]
    fn test_contains_package_with_buck_v2_file() {
        // Test that modifying a BUCK.v2 file is detected as a package change
        let buck_path = CellPath::new("root//BUCK.v2");
        let project_path = ProjectRelativePath::new("BUCK.v2");
        let paths = vec![Status::Modified((buck_path, project_path))];
        let changes = Changes::from_paths(paths);

        let package = Package::new("root//");
        assert!(
            changes.contains_package(&package),
            "BUCK.v2 file change should be detected as package change"
        );
    }

    #[test]
    fn test_contains_package_with_subdirectory_buck_file() {
        // Test that a BUCK file in a subdirectory doesn't match parent package
        let buck_path = CellPath::new("root//subdir/BUCK");
        let project_path = ProjectRelativePath::new("subdir/BUCK");
        let paths = vec![Status::Modified((buck_path, project_path))];
        let changes = Changes::from_paths(paths);

        let root_package = Package::new("root//");
        let subdir_package = Package::new("root//subdir/");

        assert!(
            !changes.contains_package(&root_package),
            "Subdirectory BUCK should not match parent package"
        );
        assert!(
            changes.contains_package(&subdir_package),
            "Subdirectory BUCK should match its own package"
        );
    }

    #[test]
    fn test_contains_package_without_buck_file() {
        // Test that non-BUCK file changes don't trigger package detection
        let source_path = CellPath::new("root//src/main.rs");
        let project_path = ProjectRelativePath::new("src/main.rs");
        let paths = vec![Status::Modified((source_path, project_path))];
        let changes = Changes::from_paths(paths);

        let package = Package::new("root//");
        assert!(
            !changes.contains_package(&package),
            "Non-BUCK file change should not be detected as package change"
        );
    }

    #[test]
    fn test_contains_package_without_trailing_slash() {
        // Test package format without trailing slash (real-world format)
        // In production, packages are typically "cell//dir" not "cell//dir/"
        let buck_path = CellPath::new("root//subdir/BUCK");
        let project_path = ProjectRelativePath::new("subdir/BUCK");
        let paths = vec![Status::Modified((buck_path, project_path))];
        let changes = Changes::from_paths(paths);

        // This is the real format used in production (no trailing slash)
        let subdir_package = Package::new("root//subdir");
        assert!(
            changes.contains_package(&subdir_package),
            "BUCK file should match package without trailing slash"
        );
    }

    #[test]
    fn test_real_world_buck_file_change_with_unresolve() {
        // This test simulates the REAL production flow:
        // Git change → ProjectRelativePath → unresolve() → CellPath → contains_package()
        // This is the flow that was FAILING in production (CL 2NY0WW96)

        let cell_info = CellInfo::testing();

        // Simulate git detecting a BUCK file change at project root
        let project_changes = vec![Status::Modified(ProjectRelativePath::new("BUCK"))];

        // This is the REAL flow that was failing before the fix
        let changes = Changes::new(&cell_info, project_changes)
            .expect("Should successfully resolve BUCK file");

        // Verify the BUCK file was successfully resolved (not skipped)
        assert!(
            !changes.is_empty(),
            "BUCK file should be resolved, not skipped"
        );

        // Verify it's detected as a package change
        let root_package = Package::new("root//");
        assert!(
            changes.contains_package(&root_package),
            "BUCK file change should trigger package change detection"
        );
    }

    #[test]
    fn test_real_world_cargo_toml_change_with_unresolve() {
        // This test simulates CL 9TNTRBBQ failure case
        let cell_info = CellInfo::testing();

        // Simulate git detecting a Cargo.toml change at project root
        let project_changes = vec![Status::Modified(ProjectRelativePath::new("Cargo.toml"))];

        let changes = Changes::new(&cell_info, project_changes)
            .expect("Should successfully resolve Cargo.toml");

        // Verify the file was successfully resolved
        assert!(
            !changes.is_empty(),
            "Cargo.toml should be resolved, not skipped"
        );

        // Verify we can find it in the changes
        let cargo_cell_path = CellPath::new("root//Cargo.toml");
        assert!(
            changes.contains_cell_path(&cargo_cell_path),
            "Cargo.toml should be in changes"
        );
    }

    #[test]
    fn test_real_world_subdirectory_file_with_unresolve() {
        // This test simulates CL OB6W18SK success case (should continue working)
        let cell_info = CellInfo::testing();

        // Simulate git detecting a source file change
        let project_changes = vec![Status::Modified(ProjectRelativePath::new("src/main.rs"))];

        let changes = Changes::new(&cell_info, project_changes)
            .expect("Should successfully resolve src/main.rs");

        // Verify the file was successfully resolved
        assert!(!changes.is_empty(), "src/main.rs should be resolved");

        // Verify we can find it in the changes
        let source_cell_path = CellPath::new("root//src/main.rs");
        assert!(
            changes.contains_cell_path(&source_cell_path),
            "src/main.rs should be in changes"
        );
    }

    #[test]
    fn test_real_world_buckconfig_change_with_unresolve() {
        // Test .buckconfig file at root (mentioned in orion.md as a config file)
        let cell_info = CellInfo::testing();

        let project_changes = vec![Status::Modified(ProjectRelativePath::new(".buckconfig"))];

        let changes = Changes::new(&cell_info, project_changes)
            .expect("Should successfully resolve .buckconfig");

        assert!(
            !changes.is_empty(),
            ".buckconfig should be resolved, not skipped"
        );

        let buckconfig_path = CellPath::new("root//.buckconfig");
        assert!(
            changes.contains_cell_path(&buckconfig_path),
            ".buckconfig should be in changes"
        );
    }

    #[test]
    fn test_real_world_multi_cell_priority() {
        // Test multi-cell environment where more specific cell should match first
        let cell_json = serde_json::json!({
            "root": ".",
            "toolchains": "./toolchains"
        });
        let cell_info = CellInfo::parse(&serde_json::to_string(&cell_json).unwrap()).unwrap();

        // File in toolchains directory should match toolchains cell, not root
        let project_changes = vec![Status::Modified(ProjectRelativePath::new(
            "toolchains/BUCK",
        ))];

        let changes = Changes::new(&cell_info, project_changes)
            .expect("Should successfully resolve toolchains/BUCK");

        assert!(!changes.is_empty(), "toolchains/BUCK should be resolved");

        // Should be in toolchains cell, not root cell
        let toolchains_path = CellPath::new("toolchains//BUCK");
        assert!(
            changes.contains_cell_path(&toolchains_path),
            "Should resolve to toolchains cell"
        );

        // Should NOT be in root cell
        let root_path = CellPath::new("root//toolchains/BUCK");
        assert!(
            !changes.contains_cell_path(&root_path),
            "Should NOT resolve to root cell"
        );
    }

    #[test]
    fn test_real_world_mixed_changes_all_resolved() {
        // Test multiple files changed at once (common in real commits)
        let cell_info = CellInfo::testing();

        let project_changes = vec![
            Status::Modified(ProjectRelativePath::new("BUCK")),
            Status::Modified(ProjectRelativePath::new("Cargo.toml")),
            Status::Modified(ProjectRelativePath::new("src/main.rs")),
            Status::Added(ProjectRelativePath::new("src/lib.rs")),
        ];

        let changes = Changes::new(&cell_info, project_changes)
            .expect("Should successfully resolve all files");

        // All 4 files should be resolved
        assert_eq!(changes.paths.len(), 4, "All 4 files should be resolved");

        // Verify each file is present
        assert!(changes.contains_cell_path(&CellPath::new("root//BUCK")));
        assert!(changes.contains_cell_path(&CellPath::new("root//Cargo.toml")));
        assert!(changes.contains_cell_path(&CellPath::new("root//src/main.rs")));
        assert!(changes.contains_cell_path(&CellPath::new("root//src/lib.rs")));

        // BUCK file should trigger package detection
        let root_package = Package::new("root//");
        assert!(
            changes.contains_package(&root_package),
            "BUCK file should trigger package detection"
        );
    }

    #[test]
    fn test_real_world_buck_v2_with_unresolve() {
        // Test BUCK.v2 variant (mentioned in orion.md)
        let cell_info = CellInfo::testing();

        let project_changes = vec![Status::Modified(ProjectRelativePath::new("BUCK.v2"))];

        let changes =
            Changes::new(&cell_info, project_changes).expect("Should successfully resolve BUCK.v2");

        assert!(!changes.is_empty(), "BUCK.v2 should be resolved");

        // Should trigger package detection
        let root_package = Package::new("root//");
        assert!(
            changes.contains_package(&root_package),
            "BUCK.v2 should trigger package detection"
        );
    }

    #[test]
    fn test_map_changes_with_resolver_filters_unresolved_paths() {
        let changes = vec![
            Status::Modified(ProjectRelativePath::new("src/main.rs")),
            Status::Added(ProjectRelativePath::new("external/shared.rs")),
        ];

        let (mapped, unresolved) = map_changes_with_resolver(changes, |path| {
            if path.as_str().starts_with("external/") {
                Err(anyhow::anyhow!("outside current cell"))
            } else {
                Ok(CellPath::new(&format!("root//{}", path.as_str())))
            }
        });

        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].get().0, CellPath::new("root//src/main.rs"));
        assert_eq!(mapped[0].get().1, ProjectRelativePath::new("src/main.rs"));
        assert_eq!(unresolved.len(), 1);
        assert_eq!(
            unresolved[0].0,
            ProjectRelativePath::new("external/shared.rs")
        );
    }

    #[test]
    fn test_changes_with_root_directory_files() {
        // 模拟实际的 cell 配置
        let cells = CellInfo::parse(r#"{"root": "."}"#).unwrap();

        let input_changes = vec![
            Status::Modified(ProjectRelativePath::new("BUCK")),
            Status::Modified(ProjectRelativePath::new("Cargo.toml")),
            Status::Modified(ProjectRelativePath::new("src/main.rs")),
        ];

        let changes = Changes::new(&cells, input_changes).unwrap();

        // 所有路径都应该成功解析
        assert_eq!(changes.paths.len(), 3, "All paths should be resolved");
        assert!(
            changes.contains_cell_path(&CellPath::new("root//BUCK")),
            "BUCK should be in changes"
        );
        assert!(
            changes.contains_cell_path(&CellPath::new("root//Cargo.toml")),
            "Cargo.toml should be in changes"
        );
        assert!(
            changes.contains_cell_path(&CellPath::new("root//src/main.rs")),
            "src/main.rs should be in changes"
        );
    }

    #[test]
    fn test_changes_preserves_status() {
        let cells = CellInfo::parse(r#"{"root": "."}"#).unwrap();

        let input_changes = vec![
            Status::Modified(ProjectRelativePath::new("BUCK")),
            Status::Added(ProjectRelativePath::new("new_file.rs")),
            Status::Removed(ProjectRelativePath::new("old_file.rs")),
        ];

        let changes = Changes::new(&cells, input_changes).unwrap();

        // 验证状态被正确保留
        let statuses: Vec<_> = changes.status_cell_paths().collect();

        // 检查每个状态类型的数量
        let modified_count = statuses
            .iter()
            .filter(|s| matches!(s, Status::Modified(_)))
            .count();
        let added_count = statuses
            .iter()
            .filter(|s| matches!(s, Status::Added(_)))
            .count();
        let removed_count = statuses
            .iter()
            .filter(|s| matches!(s, Status::Removed(_)))
            .count();

        assert_eq!(modified_count, 1, "Should have 1 modified file");
        assert_eq!(added_count, 1, "Should have 1 added file");
        assert_eq!(removed_count, 1, "Should have 1 removed file");
    }

    #[test]
    fn test_regression_cl_ob6w18sk() {
        // 确保 CL OB6W18SK 的场景仍然工作
        let cells = CellInfo::parse(r#"{"root": "."}"#).unwrap();

        let input_changes = vec![Status::Modified(ProjectRelativePath::new("src/main.rs"))];

        let changes = Changes::new(&cells, input_changes).unwrap();

        assert_eq!(changes.paths.len(), 1, "src/main.rs should be resolved");
        assert!(
            changes.contains_cell_path(&CellPath::new("root//src/main.rs")),
            "src/main.rs should be in changes"
        );
    }
}
