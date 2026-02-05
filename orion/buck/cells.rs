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
        // because we know self.paths has the longest match first, we just find the first match
        for (cell, prefix) in &self.paths {
            if let Some(x) = path.as_str().strip_prefix(prefix.as_str()) {
                let x = x.strip_prefix('/').unwrap_or(x);
                return Ok(cell.join(&CellRelativePath::new(x)));
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
}
