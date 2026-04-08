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
use td_util::prelude::*;
use td_util_buck::{
    cells::CellInfo,
    types::{CellPath, Package},
};

#[derive(Default, Debug)]
pub struct Changes {
    paths: Vec<Status<(CellPath, ProjectRelativePath)>>,
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

    fn from_paths(paths: Vec<Status<(CellPath, ProjectRelativePath)>>) -> Self {
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

        let paths = changes.map(|x| x.map(|x| (x.clone(), mk_project_path(x))));
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
        self.contains_cell_path(&package.as_cell_path())
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
) -> (
    Vec<Status<(CellPath, ProjectRelativePath)>>,
    Vec<(ProjectRelativePath, anyhow::Error)>,
) {
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
}
