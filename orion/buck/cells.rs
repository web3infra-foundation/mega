/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::{collections::HashMap, fs, path::Path};

use anyhow::Context as _;
use api_model::buck2::types::ProjectRelativePath;
use itertools::Itertools;
use td_util::prelude::*;
use thiserror::Error;

use crate::{
    ignore_set::IgnoreSet,
    types::{CellName, CellPath, CellRelativePath},
};

/// The value of `buildfile.name` if omitted.
const DEFAULT_BUILD_FILES: &[&str] = &["BUCK.v2", "BUCK"];

#[derive(Debug)]
struct CellData {
    path: ProjectRelativePath,
    build_files: Vec<String>,
    ignore: IgnoreSet,
}

impl CellData {
    fn new(path: ProjectRelativePath) -> Self {
        Self {
            path,
            build_files: DEFAULT_BUILD_FILES.map(|x| (*x).to_owned()),
            ignore: IgnoreSet::default(),
        }
    }

    fn set_build_files(&mut self, value: &str, infer_v2: bool) {
        self.build_files.clear();
        for x in value.split(',').map(str::trim) {
            if infer_v2 {
                self.build_files.push(format!("{x}.v2"));
            }
            self.build_files.push(x.to_owned());
        }
    }

    fn set_ignore(&mut self, value: &str) {
        self.ignore = IgnoreSet::new(value);
    }
}

#[derive(Debug)]
pub struct CellInfo {
    cells: HashMap<CellName, CellData>,
    /// Sorted by path length, so the longest is first
    paths: Vec<(CellName, ProjectRelativePath)>,
}

#[derive(Error, Debug)]
enum CellError {
    #[error("Unknown cell, `{0}`")]
    UnknownCell(CellPath),
    #[error("Path has no cell which is a prefix `{0}`")]
    UnknownPath(ProjectRelativePath),
    #[error("Empty JSON object for the cells")]
    EmptyJson,
    #[error("Expected key `{key}` to start with `{prefix}`, but got `{value}`")]
    InvalidKey {
        key: String,
        prefix: String,
        value: String,
    },
}

impl CellInfo {
    /// A default `CellInfo` for use in tests.
    pub fn testing() -> Self {
        // We'd really like this to be `#[cfg(any(test, doctest))]`, but that doesn't work
        // because of https://github.com/rust-lang/rust/issues/67295.

        // Some sample values that we use in various tests, roughly modelled on internal defaults.
        let value = serde_json::json!(
            {
                "root": "/Users/ndmitchell/repo",
                "foo": "/Users/ndmitchell/repo/foo",
                "bar": "/Users/ndmitchell/repo/bar",
                "fbcode": "/Users/ndmitchell/repo/fbcode",
                "prelude": "/Users/ndmitchell/repo/fbcode/prelude",
              }
        );
        let mut res = CellInfo::parse(&value.to_string()).unwrap();
        let config = serde_json::json!(
            {
                "fbcode//buildfile.name":"TARGETS",
                "prelude//buildfile.name":"TARGETS",
            }
        );
        res.parse_config_data(&config.to_string()).unwrap();
        res
    }

    pub fn new(file: &Path) -> anyhow::Result<Self> {
        let data = fs::read_to_string(file)
            .with_context(|| format!("When reading `{}`", file.display()))?;
        Self::parse(&data)
    }

    fn parse_cells_data(data: &str) -> anyhow::Result<HashMap<CellName, CellData>> {
        let json: HashMap<String, String> = serde_json::from_str(data)?;

        // We need to find the shortest path, as that will be the prefix and we want project relative paths
        let prefix = json
            .values()
            .min_by_key(|x| x.len())
            .ok_or(CellError::EmptyJson)?
            .to_owned();
        let mut cells = HashMap::with_capacity(json.len());
        for (k, v) in json.into_iter() {
            match v.strip_prefix(&prefix) {
                None => {
                    return Err(CellError::InvalidKey {
                        key: k,
                        prefix,
                        value: v,
                    }
                    .into());
                }
                Some(rest) => {
                    cells.insert(
                        CellName::new(&k),
                        CellData::new(ProjectRelativePath::new(rest.trim_start_matches('/'))),
                    );
                }
            }
        }
        Ok(cells)
    }

    fn create_paths(cells: &HashMap<CellName, CellData>) -> Vec<(CellName, ProjectRelativePath)> {
        let mut paths = cells
            .iter()
            .map(|(k, v)| ((*k).clone(), v.path.clone()))
            .collect::<Vec<_>>();
        paths.sort_by_key(|x| -(x.1.as_str().len() as isize));
        paths
    }

    pub fn parse(data: &str) -> anyhow::Result<Self> {
        let cells = Self::parse_cells_data(data)?;
        let paths = Self::create_paths(&cells);
        Ok(Self { cells, paths })
    }

    pub fn load_config_data(&mut self, file: &Path) -> anyhow::Result<()> {
        let data = fs::read_to_string(file)
            .with_context(|| format!("When reading `{}`", file.display()))?;
        self.parse_config_data(&data)
    }

    pub fn parse_config_data(&mut self, data: &str) -> anyhow::Result<()> {
        let json: HashMap<String, String> = serde_json::from_str(data)?;

        // The keys have names like `cell//buildfile.name`.
        // We sort and group by cell name, so we only see each cell name once,
        // and so that `buildfile.name_v2` is _after_ `buildfile.name` as it must always take preference.
        let mut xs = json
            .iter()
            .filter_map(|(k, v)| {
                let (cell, k) = k.split_once("//")?;
                Some((cell, (k, v)))
            })
            .collect::<Vec<_>>();
        xs.sort();
        for (cell, items) in &xs.iter().chunk_by(|x| x.0) {
            let cell = CellName::new(cell);
            let cell_data = match self.cells.get_mut(&cell) {
                Some(data) => data,
                None => return Err(CellError::UnknownCell(cell.as_cell_path()).into()),
            };

            for (_cell, (key, value)) in items {
                match *key {
                    "buildfile.name" => cell_data.set_build_files(value, true),
                    "buildfile.name_v2" => cell_data.set_build_files(value, false),
                    "project.ignore" => cell_data.set_ignore(value),
                    _ => {
                        // Extra config isn't a problem, just ignore it
                    }
                }
            }
        }
        Ok(())
    }

    pub fn resolve(&self, path: &CellPath) -> anyhow::Result<ProjectRelativePath> {
        match self.cells.get(&path.cell()) {
            Some(data) => Ok(data.path.join(path.path().as_str())),
            None => Err(CellError::UnknownCell(path.clone()).into()),
        }
    }

    pub fn unresolve(&self, path: &ProjectRelativePath) -> anyhow::Result<CellPath> {
        let path_str = path.as_str();

        // self.paths is sorted by path length (longest first) for priority matching
        for (cell, prefix) in &self.paths {
            let prefix_str = prefix.as_str();
            // Normalize: remove trailing slashes from prefix for consistent matching
            let prefix_normalized = prefix_str.trim_end_matches('/');

            // Special handling: empty prefix or "." represents the root cell
            // Semantically, it means "match all paths that don't have a more specific prefix"
            if prefix_normalized.is_empty() || prefix_normalized == "." {
                // Check if there's a more specific prefix that matches this path
                let has_more_specific_match = self.paths.iter().any(|(_, other_prefix)| {
                    let other_str = other_prefix.as_str().trim_end_matches('/');
                    !other_str.is_empty()
                        && other_str != "."
                        && (path_str == other_str
                            || path_str.starts_with(&format!("{}/", other_str)))
                });

                // If no more specific match exists, this path belongs to the root cell
                if !has_more_specific_match {
                    tracing::debug!(
                        "Resolved path '{}' to root cell '{}' (prefix: '{}')",
                        path_str,
                        cell.as_str(),
                        prefix_str
                    );
                    return Ok(cell.join(&CellRelativePath::new(path_str)));
                }
            } else {
                // Non-root cell: use standard prefix matching
                if path_str == prefix_normalized {
                    // Exact match: path is the cell's root directory
                    return Ok(cell.join(&CellRelativePath::new("")));
                } else if let Some(x) = path_str.strip_prefix(&format!("{}/", prefix_normalized)) {
                    // Prefix match: path is in a subdirectory of the cell
                    return Ok(cell.join(&CellRelativePath::new(x)));
                }
            }
        }

        Err(CellError::UnknownPath(path.clone()).into())
    }

    pub fn build_files(&self, cell: &CellName) -> anyhow::Result<&[String]> {
        match self.cells.get(cell) {
            Some(data) => Ok(&data.build_files),
            None => Err(CellError::UnknownCell(cell.as_cell_path()).into()),
        }
    }

    pub fn is_ignored(&self, path: &CellPath) -> bool {
        match self.cells.get(&path.cell()) {
            None => false,
            Some(data) => data.ignore.is_match(path.path().as_str()),
        }
    }

    /// Returns target patterns for all cells (e.g., ["root//...", ...])
    /// This is used to query targets from all cells, not just the root cell.
    ///
    /// Note: Excludes special cells:
    /// - "prelude": Contains build rule definitions, may have parsing issues
    /// - "none": Placeholder cell used for cell aliases, doesn't contain actual build targets
    ///
    /// Also excludes cells whose directories don't exist in the project root.
    /// This prevents errors when querying cells that are defined in .buckconfig
    /// but don't have corresponding directories (e.g., toolchains in test fixtures).
    pub fn get_all_cell_patterns(&self, project_root: &Path) -> Vec<String> {
        self.cells
            .iter()
            .filter(|(cell_name, cell_data)| {
                let cell_str = cell_name.as_str();

                // Exclude known special/placeholder cells
                if cell_str == "prelude" || cell_str == "none" {
                    return false;
                }

                // Check if the cell directory actually exists
                let cell_path = project_root.join(cell_data.path.as_str());
                if !cell_path.exists() {
                    tracing::debug!(
                        "Excluding cell '{}' from query: directory {:?} does not exist",
                        cell_str,
                        cell_path
                    );
                    return false;
                }

                true
            })
            .map(|(cell_name, _)| format!("{}//...", cell_name.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell() {
        let value = serde_json::json!(
            {
                "inner1": "/Users/ndmitchell/repo/inner1",
                "inner2": "/Users/ndmitchell/repo/inner1/inside/inner2",
                "root": "/Users/ndmitchell/repo",
                "prelude": "/Users/ndmitchell/repo/prelude"
              }
        );
        let cells = CellInfo::parse(&serde_json::to_string(&value).unwrap()).unwrap();

        fn testcase(cells: &CellInfo, cell_path: &str, project_relative_path: &str) {
            let cell_path = CellPath::new(cell_path);
            let project_relative_path = ProjectRelativePath::new(project_relative_path);
            assert_eq!(cells.resolve(&cell_path).unwrap(), project_relative_path);
            assert_eq!(cells.unresolve(&project_relative_path).unwrap(), cell_path);
        }

        testcase(&cells, "inner1//magic/file.txt", "inner1/magic/file.txt");
        testcase(
            &cells,
            "inner2//magic/file.txt",
            "inner1/inside/inner2/magic/file.txt",
        );
        testcase(&cells, "root//file.txt", "file.txt");

        assert!(cells.resolve(&CellPath::new("missing//foo.txt")).is_err());
    }

    #[test]
    fn test_cell_config() {
        let value = serde_json::json!(
            {
                "root": "/Users/ndmitchell/repo",
                "cell1": "/Users/ndmitchell/repo/cell1",
                "cell2": "/Users/ndmitchell/repo/cell2",
                "cell3": "/Users/ndmitchell/repo/cell3",
              }
        );
        let mut cells = CellInfo::parse(&serde_json::to_string(&value).unwrap()).unwrap();
        let value = serde_json::json!(
            {
                "cell1//buildfile.name":"BUCK",
                "cell1//buildfile.name_v2":"TARGETS",
                "cell2//buildfile.name":"A1,A2",
            }
        );
        cells
            .parse_config_data(&serde_json::to_string(&value).unwrap())
            .unwrap();
        assert_eq!(
            cells.build_files(&CellName::new("cell1")).unwrap(),
            &["TARGETS"]
        );
        assert_eq!(
            cells.build_files(&CellName::new("cell2")).unwrap(),
            &["A1.v2", "A1", "A2.v2", "A2"]
        );
        assert_eq!(
            cells.build_files(&CellName::new("cell3")).unwrap(),
            &["BUCK.v2", "BUCK"]
        );
        assert!(cells.build_files(&CellName::new("cell4")).is_err());
    }

    #[test]
    fn test_cell_config_incompatible() {
        let value = serde_json::json!(
            {
                "root": "/Users/ndmitchell/repo",
            }
        );
        let mut cells = CellInfo::parse(&value.to_string()).unwrap();
        let config = serde_json::json!(
            {
                "cell1//buildfile.name":"BUILD",
            }
        );
        assert!(cells.parse_config_data(&config.to_string()).is_err());
    }

    #[test]
    fn test_cell_ignore() {
        let value = serde_json::json!(
            {
                "root": "/Users/ndmitchell/repo",
                "cell1": "/Users/ndmitchell/repo/cell1",
            }
        );
        let mut cells = CellInfo::parse(&value.to_string()).unwrap();
        let config = serde_json::json!(
            {
                "cell1//project.ignore":"bar/baz",
            }
        );
        cells.parse_config_data(&config.to_string()).unwrap();

        assert!(!cells.is_ignored(&CellPath::new("root//bar/baz/file.txt")));
        assert!(cells.is_ignored(&CellPath::new("cell1//bar/baz/file.txt")));
        assert!(!cells.is_ignored(&CellPath::new("root//cell1/bar/baz/file.txt")));
        assert!(!cells.is_ignored(&CellPath::new("cell1//cell1/bar/baz/file.txt")));
    }

    #[test]
    fn test_unresolve_root_cell_with_dot_prefix() {
        // 模拟实际的 cell 配置：root = .
        let cells = CellInfo::parse(r#"{"root": "."}"#).unwrap();

        // Case 1: BUCK 文件（CL 2NY0WW96 - 当前失败）
        let result = cells.unresolve(&ProjectRelativePath::new("BUCK"));
        assert!(result.is_ok(), "BUCK should be resolved to root cell");
        assert_eq!(result.unwrap(), CellPath::new("root//BUCK"));

        // Case 2: Cargo.toml（CL 9TNTRBBQ - 当前失败）
        let result = cells.unresolve(&ProjectRelativePath::new("Cargo.toml"));
        assert!(result.is_ok(), "Cargo.toml should be resolved to root cell");
        assert_eq!(result.unwrap(), CellPath::new("root//Cargo.toml"));

        // Case 3: .buckconfig（配置文件）
        let result = cells.unresolve(&ProjectRelativePath::new(".buckconfig"));
        assert!(
            result.is_ok(),
            ".buckconfig should be resolved to root cell"
        );
        assert_eq!(result.unwrap(), CellPath::new("root//.buckconfig"));

        // Case 4: src/main.rs（CL OB6W18SK - 回归测试，应该继续工作）
        let result = cells.unresolve(&ProjectRelativePath::new("src/main.rs"));
        assert!(
            result.is_ok(),
            "src/main.rs should be resolved to root cell"
        );
        assert_eq!(result.unwrap(), CellPath::new("root//src/main.rs"));

        // Case 5: 空路径
        let result = cells.unresolve(&ProjectRelativePath::new(""));
        assert!(result.is_ok(), "empty path should be resolved to root cell");
        assert_eq!(result.unwrap(), CellPath::new("root//"));
    }

    #[test]
    fn test_unresolve_cell_with_trailing_slash() {
        // Test that cells with trailing slashes in JSON are handled correctly
        let cell_json = serde_json::json!({
            "root": ".",
            "toolchains": "./toolchains/"  // Note: trailing slash
        });
        let cell_info = CellInfo::parse(&serde_json::to_string(&cell_json).unwrap()).unwrap();

        // File in toolchains directory should match toolchains cell
        let result = cell_info.unresolve(&ProjectRelativePath::new("toolchains/BUCK"));
        assert!(result.is_ok(), "Should resolve toolchains/BUCK");

        let cell_path = result.unwrap();
        assert_eq!(
            cell_path.as_str(),
            "toolchains//BUCK",
            "Should resolve to toolchains cell, not root"
        );
    }

    #[test]
    fn test_unresolve_multi_cell_priority() {
        // 模拟多 cell 配置，使用绝对路径格式
        // 在实际场景中，. 会被解析为某个绝对路径，这里用 /repo 模拟
        let cells = CellInfo::parse(
            r#"{
            "root": "/repo",
            "toolchains": "/repo/toolchains"
        }"#,
        )
        .unwrap();

        // Case 1: toolchains 目录下的文件应该匹配 toolchains cell（更具体的匹配）
        let result = cells.unresolve(&ProjectRelativePath::new("toolchains/BUCK"));
        assert!(
            result.is_ok(),
            "toolchains/BUCK should be resolved to toolchains cell"
        );
        assert_eq!(result.unwrap(), CellPath::new("toolchains//BUCK"));

        // Case 2: 根目录的 BUCK 应该匹配 root cell
        let result = cells.unresolve(&ProjectRelativePath::new("BUCK"));
        assert!(result.is_ok(), "BUCK should be resolved to root cell");
        assert_eq!(result.unwrap(), CellPath::new("root//BUCK"));

        // Case 3: src 目录下的文件应该匹配 root cell（没有更具体的匹配）
        let result = cells.unresolve(&ProjectRelativePath::new("src/main.rs"));
        assert!(
            result.is_ok(),
            "src/main.rs should be resolved to root cell"
        );
        assert_eq!(result.unwrap(), CellPath::new("root//src/main.rs"));
    }

    #[test]
    fn test_get_all_cell_patterns() {
        use std::env;
        use std::fs;

        let temp_dir = env::temp_dir().join("buck_cells_test_1");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
        fs::create_dir_all(&temp_dir).unwrap();
        fs::create_dir_all(temp_dir.join("toolchains")).unwrap();
        fs::create_dir_all(temp_dir.join("prelude")).unwrap();

        let cell_json = serde_json::json!({
            "root": temp_dir.to_str().unwrap(),
            "toolchains": temp_dir.join("toolchains").to_str().unwrap(),
            "prelude": temp_dir.join("prelude").to_str().unwrap()
        });
        let cells = CellInfo::parse(&serde_json::to_string(&cell_json).unwrap()).unwrap();

        let patterns = cells.get_all_cell_patterns(&temp_dir);

        // Should have patterns for root and toolchains (both exist), but not prelude (special)
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"root//...".to_string()));
        assert!(patterns.contains(&"toolchains//...".to_string()));
        assert!(!patterns.contains(&"prelude//...".to_string()));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_all_cell_patterns_excludes_none_placeholder() {
        use std::env;
        use std::fs;

        let temp_dir = env::temp_dir().join("buck_cells_test_2");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
        fs::create_dir_all(&temp_dir).unwrap();
        fs::create_dir_all(temp_dir.join("toolchains")).unwrap();
        fs::create_dir_all(temp_dir.join("prelude")).unwrap();
        fs::create_dir_all(temp_dir.join("none")).unwrap();

        let cell_json = serde_json::json!({
            "root": temp_dir.to_str().unwrap(),
            "toolchains": temp_dir.join("toolchains").to_str().unwrap(),
            "prelude": temp_dir.join("prelude").to_str().unwrap(),
            "none": temp_dir.join("none").to_str().unwrap()
        });
        let cells = CellInfo::parse(&serde_json::to_string(&cell_json).unwrap()).unwrap();

        let patterns = cells.get_all_cell_patterns(&temp_dir);

        // Should exclude prelude (special) and none (placeholder), even though directories exist
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"root//...".to_string()));
        assert!(patterns.contains(&"toolchains//...".to_string()));
        assert!(!patterns.contains(&"prelude//...".to_string()));
        assert!(!patterns.contains(&"none//...".to_string()));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_all_cell_patterns_excludes_nonexistent_dirs() {
        use std::env;
        use std::fs;

        let temp_dir = env::temp_dir().join("buck_cells_test_3");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
        fs::create_dir_all(&temp_dir).unwrap();
        // Only create root, not toolchains

        let cell_json = serde_json::json!({
            "root": temp_dir.to_str().unwrap(),
            "toolchains": temp_dir.join("toolchains").to_str().unwrap(),
        });
        let cells = CellInfo::parse(&serde_json::to_string(&cell_json).unwrap()).unwrap();

        let patterns = cells.get_all_cell_patterns(&temp_dir);

        // Should only have root, since toolchains directory doesn't exist
        assert_eq!(patterns.len(), 1);
        assert!(patterns.contains(&"root//...".to_string()));
        assert!(!patterns.contains(&"toolchains//...".to_string()));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
