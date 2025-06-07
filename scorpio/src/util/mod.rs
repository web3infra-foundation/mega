use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};
pub mod config;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GPath {
    pub path: Vec<String>,
}

impl GPath {
    pub fn new() -> GPath {
        GPath { path: Vec::new() }
    }
    pub fn push(&mut self, path: String) {
        if path.contains('/') {
            for part in path.split('/') {
                if !part.is_empty() {
                    self.path.push(part.to_string());
                }
            }
        } else {
            self.path.push(path);
        }
    }
    pub fn pop(&mut self) -> Option<String> {
        self.path.pop()
    }
    pub fn name(&self) -> String {
        self.path.last().unwrap().clone()
    }
    pub fn part(&self, i: usize, j: usize) -> String {
        self.path[i..j].join("/")
    }
}

impl From<String> for GPath {
    fn from(mut s: String) -> GPath {
        if s.starts_with('/') {
            s.remove(0);
        }
        GPath {
            path: s.split('/').map(String::from).collect(),
        }
    }
}

impl From<GPath> for PathBuf {
    fn from(val: GPath) -> Self {
        let path_str = val.path.join("/");
        PathBuf::from(path_str)
    }
}
impl From<GPath> for String {
    fn from(mut val: GPath) -> Self {
        val.path.retain(|part| !part.is_empty());
        val.path.join("/")
    }
}
impl Display for GPath {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.path.join("/"))
    }
}

/// Turn a path to a relative path to the working directory
/// - not check existence
pub fn to_workdir_path(path: impl AsRef<Path>) -> PathBuf {
    let p = path.as_ref();
    let binding = config::workspace();
    let workspace = std::path::Path::new(&binding);
    if let Ok(relative) = p.strip_prefix(workspace) {
        relative.to_path_buf()
    } else {
        p.to_path_buf()
    }
}

pub fn from_store_path_to_workdir(path: impl AsRef<Path>) -> PathBuf {
    let p = path.as_ref();
    let binding = config::store_path();
    let workspace = std::path::Path::new(&binding);
    if let Ok(relative) = p.strip_prefix(workspace) {
        relative.to_path_buf()
    } else {
        p.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::GPath;

    #[test]
    fn test_from_string() {
        let path = String::from("/release");
        let gapth = GPath::from(path);
        assert_eq!(gapth.to_string(), String::from("release"))
    }
}
