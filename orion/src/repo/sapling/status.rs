/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

// use std::{fs, path::Path};

// use anyhow::Context as _;
// pub use td_util_buck::types::ProjectRelativePath;
// use thiserror::Error;
// use utoipa::ToSchema;

// #[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
// pub enum Status<Path> {
//     Modified(Path),
//     Added(Path),
//     Removed(Path),
// }

// #[derive(Error, Debug)]
// enum StatusParseError {
//     #[error("Unexpected line format: {0}")]
//     UnexpectedFormat(String),
//     #[error("Unknown line prefix: {0}")]
//     UnknownPrefix(String),
// }

// impl Status<ProjectRelativePath> {
//     /// Creates a new Modified status from a file path string
//     pub fn modified(path: &str) -> Self {
//         Self::Modified(ProjectRelativePath::new(path))
//     }

//     /// Creates a new Added status from a file path string
//     pub fn added(path: &str) -> Self {
//         Self::Added(ProjectRelativePath::new(path))
//     }

//     /// Creates a new Removed status from a file path string
//     pub fn removed(path: &str) -> Self {
//         Self::Removed(ProjectRelativePath::new(path))
//     }

//     fn from_str(value: &str) -> anyhow::Result<Self> {
//         let mut it = value.chars();
//         let typ = it.next();
//         if it.next() != Some(' ') {
//             return Err(StatusParseError::UnexpectedFormat(value.to_owned()).into());
//         }
//         let path = ProjectRelativePath::new(it.as_str());
//         match typ {
//             Some('A') => Ok(Self::Added(path)),
//             Some('M') => Ok(Self::Modified(path)),
//             Some('R') => Ok(Self::Removed(path)),
//             Some('D') => Ok(Self::Removed(path)), // used by jujutsu
//             _ => Err(StatusParseError::UnknownPrefix(value.to_owned()).into()),
//         }
//     }
// }

// impl<Path> Status<Path> {
//     pub fn get(&self) -> &Path {
//         match self {
//             Status::Modified(x) => x,
//             Status::Added(x) => x,
//             Status::Removed(x) => x,
//         }
//     }

//     pub fn map<'a, T: 'a>(&'a self, f: impl FnOnce(&'a Path) -> T) -> Status<T> {
//         match self {
//             Status::Modified(x) => Status::Modified(f(x)),
//             Status::Added(x) => Status::Added(f(x)),
//             Status::Removed(x) => Status::Removed(f(x)),
//         }
//     }

//     pub fn try_map<T, E>(&self, f: impl FnOnce(&Path) -> Result<T, E>) -> Result<Status<T>, E> {
//         Ok(match self {
//             Status::Modified(x) => Status::Modified(f(x)?),
//             Status::Added(x) => Status::Added(f(x)?),
//             Status::Removed(x) => Status::Removed(f(x)?),
//         })
//     }

//     pub fn into_map<T>(self, f: impl FnOnce(Path) -> T) -> Status<T> {
//         match self {
//             Status::Modified(x) => Status::Modified(f(x)),
//             Status::Added(x) => Status::Added(f(x)),
//             Status::Removed(x) => Status::Removed(f(x)),
//         }
//     }

//     pub fn into_try_map<T, E>(self, f: impl FnOnce(Path) -> Result<T, E>) -> Result<Status<T>, E> {
//         Ok(match self {
//             Status::Modified(x) => Status::Modified(f(x)?),
//             Status::Added(x) => Status::Added(f(x)?),
//             Status::Removed(x) => Status::Removed(f(x)?),
//         })
//     }
// }

// pub fn read_status(path: &Path) -> anyhow::Result<Vec<Status<ProjectRelativePath>>> {
//     parse_status(
//         &fs::read_to_string(path).with_context(|| format!("When reading `{}`", path.display()))?,
//     )
// }

// fn parse_status(data: &str) -> anyhow::Result<Vec<Status<ProjectRelativePath>>> {
//     data.lines()
//         .map(Status::from_str)
//         .collect::<anyhow::Result<Vec<_>>>()
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_status() {
//         let src = r#"
// M proj/foo.rs
// M bar.rs
// A baz/file.txt
// R quux.js
// "#;
//         assert_eq!(
//             parse_status(&src[1..]).unwrap(),
//             vec![
//                 Status::Modified(ProjectRelativePath::new("proj/foo.rs")),
//                 Status::Modified(ProjectRelativePath::new("bar.rs")),
//                 Status::Added(ProjectRelativePath::new("baz/file.txt")),
//                 Status::Removed(ProjectRelativePath::new("quux.js"))
//             ]
//         );
//     }

//     #[test]
//     fn test_status_error() {
//         assert!(parse_status("X quux.js").is_err());
//         assert!(parse_status("notaline").is_err());
//         assert!(parse_status("not a line").is_err());
//     }

//     #[test]
//     fn test_status_constructors() {
//         let modified = Status::modified("foo/modified.rs");
//         let modified_parsed = Status::from_str("M foo/modified.rs").unwrap();
//         assert!(matches!(modified, Status::Modified(_)));
//         assert_eq!(modified, modified_parsed);
//         assert_eq!(modified.get().as_str(), "foo/modified.rs");

//         let added = Status::added("foo/added.rs");
//         let added_parsed = Status::from_str("A foo/added.rs").unwrap();
//         assert!(matches!(added, Status::Added(_)));
//         assert_eq!(added, added_parsed);
//         assert_eq!(added.get().as_str(), "foo/added.rs");

//         let removed = Status::removed("foo/removed.rs");
//         let removed_parsed = Status::from_str("R foo/removed.rs").unwrap();
//         assert!(matches!(removed, Status::Removed(_)));
//         assert_eq!(removed, removed_parsed);
//         assert_eq!(removed.get().as_str(), "foo/removed.rs");
//     }
// }
