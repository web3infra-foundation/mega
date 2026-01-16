/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use serde::{Deserialize, Serialize};
use td_util::json;

use crate::{
    labels::Labels,
    types::{
        CellPath, Glob, Oncall, Package, PackageValues, RuleType, TargetHash, TargetLabel,
        TargetLabelKeyRef, TargetName, TargetPattern,
    },
};

/// The output of running `buck2 targets`.
#[derive(Clone)]
pub struct Targets(Vec<TargetsEntry>);

impl Targets {
    pub fn from_file(file: &Path) -> anyhow::Result<Targets> {
        Ok(Self(json::read_file_lines_parallel(file)?))
    }

    pub fn new(entries: Vec<TargetsEntry>) -> Self {
        Self(entries)
    }

    /// Return the upperbound of `self.targets().count()`.
    /// Usually a fairly close approximation, as most entries are targets.
    pub fn len_targets_upperbound(&self) -> usize {
        self.0.len()
    }

    /// Create a map from target key to target
    pub fn targets_by_label_key(&self) -> HashMap<TargetLabelKeyRef<'_>, &BuckTarget> {
        let mut res = HashMap::with_capacity(self.len_targets_upperbound());
        for x in self.targets() {
            res.insert(x.label_key(), x);
        }
        res
    }

    /// Create a map from target label to target
    pub fn targets_by_label(&self) -> HashMap<TargetLabel, &BuckTarget> {
        let mut res = HashMap::with_capacity(self.len_targets_upperbound());
        for x in self.targets() {
            res.insert(x.label(), x);
        }
        res
    }

    /// Replace the packages with those from `new`
    pub fn update(&self, mut new: Targets, removed: &HashSet<Package>) -> Self {
        if new.0.is_empty() && removed.is_empty() {
            // Fast path - nothing has changed
            return self.clone();
        }

        // Figure out what `new` replaces
        let mut replaced_package = HashSet::new();
        let mut replaced_import = HashSet::new();
        for x in new.imports() {
            match &x.package {
                Some(pkg) => replaced_package.insert(pkg.clone()),
                None => replaced_import.insert(x.file.clone()),
            };
        }
        // Copy things from self, where they still make sense
        for x in self.entries() {
            let replaced = match x {
                TargetsEntry::Target(x) => {
                    removed.contains(&x.package) || replaced_package.contains(&x.package)
                }
                TargetsEntry::Error(x) => {
                    removed.contains(&x.package) || replaced_package.contains(&x.package)
                }
                TargetsEntry::Import(x) => match &x.package {
                    Some(pkg) => removed.contains(pkg) || replaced_package.contains(pkg),
                    None => replaced_import.contains(&x.file),
                },
            };
            if !replaced {
                new.0.push(x.clone());
            }
        }
        new
    }

    pub fn entries(&self) -> impl Iterator<Item = &TargetsEntry> {
        self.0.iter()
    }

    pub fn targets(&self) -> impl Iterator<Item = &BuckTarget> {
        self.0.iter().filter_map(|x| match x {
            TargetsEntry::Target(x) => Some(x),
            _ => None,
        })
    }

    pub fn imports(&self) -> impl Iterator<Item = &BuckImport> {
        self.0.iter().filter_map(|x| match x {
            TargetsEntry::Import(x) => Some(x),
            _ => None,
        })
    }

    pub fn errors(&self) -> impl Iterator<Item = &BuckError> {
        self.0.iter().filter_map(|x| match x {
            TargetsEntry::Error(x) => Some(x),
            _ => None,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum TargetsEntry {
    Target(BuckTarget),
    Import(BuckImport),
    Error(BuckError),
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct BuckTarget {
    /// Name of the target
    pub name: TargetName,
    /// Package in which the name exists
    #[serde(rename = "buck.package")]
    pub package: Package,
    #[serde(
        rename = "buck.package_values",
        default,
        skip_serializing_if = "PackageValues::is_empty"
    )]
    pub package_values: PackageValues,
    /// Custom type, e.g. cpp rule
    #[serde(rename = "buck.type")]
    pub rule_type: RuleType,
    /// The name of the oncall for this target, if it exists
    #[serde(
        rename = "buck.oncall",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub oncall: Option<Oncall>,
    /// Its dependencies (buck.deps attribute)
    #[serde(rename = "buck.deps")]
    pub deps: Box<[TargetLabel]>,
    /// Source files used by this targets fbcode//a/c.cpp
    #[serde(rename = "buck.inputs")]
    pub inputs: Box<[CellPath]>,
    /// Hash values of all attributes, not including file content
    #[serde(rename = "buck.target_hash")]
    pub hash: TargetHash,
    /// A target can have multiple labels
    #[serde(default, skip_serializing_if = "Labels::is_empty")]
    pub labels: Labels,
    /// Used as additional triggers
    #[serde(default, skip_serializing_if = "is_empty_slice")]
    pub ci_srcs: Box<[Glob]>,
    /// Used as additional triggers
    #[serde(default, skip_serializing_if = "is_empty_slice")]
    pub ci_srcs_must_match: Box<[Glob]>,
    /// Used as additional triggers
    #[serde(default, skip_serializing_if = "is_empty_slice")]
    pub ci_deps: Box<[TargetPattern]>,
}

fn is_empty_slice<T>(x: &[T]) -> bool {
    x.is_empty()
}

impl BuckTarget {
    pub fn label(&self) -> TargetLabel {
        self.package.join(&self.name)
    }

    pub fn label_key(&self) -> TargetLabelKeyRef<'_> {
        TargetLabelKeyRef::new(&self.package, &self.name)
    }

    pub fn testing(name: &str, package: &str, rule_type: &str) -> BuckTarget {
        Self {
            name: TargetName::new(name),
            package: Package::new(package),
            package_values: PackageValues::default(),
            deps: Box::new([]),
            inputs: Box::new([]),
            rule_type: RuleType::new(rule_type),
            hash: TargetHash::new("123abc"),
            labels: Labels::default(),
            oncall: None,
            ci_srcs: Box::new([]),
            ci_srcs_must_match: Box::new([]),
            ci_deps: Box::new([]),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct BuckError {
    // Error in starlark + package file
    #[serde(rename = "buck.package")]
    pub package: Package,
    #[serde(rename = "buck.error")]
    pub error: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct BuckImport {
    /// Path of this file
    #[serde(rename = "buck.file")]
    pub file: CellPath,
    /// a list of paths that this target imports
    #[serde(rename = "buck.imports")]
    pub imports: Box<[CellPath]>,
    /// e.g. fbcode//buck2, every TARGETS file lives in a package, otherwise set to None, e.g. for .bzl/PACKAGE files
    #[serde(rename = "buck.package")]
    pub package: Option<Package>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use td_util::prelude::*;
    use tempfile::NamedTempFile;

    use super::*;

    fn write_buck_input(value: Value) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            value
                .as_array()
                .unwrap()
                .map(|x| serde_json::to_string(&x).unwrap())
                .join("\n"),
        )
        .unwrap();
        file
    }

    #[test]
    fn test_read_targets() {
        let value = serde_json::json!(
            [
                {
                    "buck.package": "fbcode//pkg",
                    "buck.file": "fbcode//pkg/TARGETS",
                    "buck.imports": ["prelude//prelude.bzl", "fbcode//infra/defs.bzl"]
                },
                {
                    "buck.type": "prelude//rules.bzl:python_library",
                    "buck.deps": ["toolchains//:python", "fbcode//python:library"],
                    "buck.inputs": ["fbcode//me/file.bzl"],
                    "buck.target_hash": "43ce1a7a56f10225413a2991febb853a",
                    "buck.package": "fbcode//me",
                    "buck.package_values": {"citadel.labels": ["ci:@fbcode//mode/opt"]},
                    "name": "test",
                },
                {
                    "buck.type": "prelude//rules.bzl:cxx_library",
                    "buck.deps": [],
                    "buck.inputs": [],
                    "buck.target_hash": "413a2991febb853a43ce1a7a56f10225",
                    "buck.oncall": "my_team",
                    "buck.package": "fbcode//me",
                    "buck.package_values": {},
                    "name": "test2",
                    "labels": ["my_label"]
                },
                {
                    "buck.package": "fbcode//broken",
                    "buck.error": "Broken :("
                },
            ]
        );
        let file = write_buck_input(value);

        let res = Targets::from_file(file.path()).unwrap();
        let required = vec![
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("fbcode//pkg/TARGETS"),
                imports: Box::new([
                    CellPath::new("prelude//prelude.bzl"),
                    CellPath::new("fbcode//infra/defs.bzl"),
                ]),
                package: Some(Package::new("fbcode//pkg")),
            }),
            TargetsEntry::Target(BuckTarget {
                deps: Box::new([
                    TargetLabel::new("toolchains//:python"),
                    TargetLabel::new("fbcode//python:library"),
                ]),
                inputs: Box::new([CellPath::new("fbcode//me/file.bzl")]),
                hash: TargetHash::new("43ce1a7a56f10225413a2991febb853a"),
                package_values: PackageValues::new(&["ci:@fbcode//mode/opt"]),
                ..BuckTarget::testing("test", "fbcode//me", "prelude//rules.bzl:python_library")
            }),
            TargetsEntry::Target(BuckTarget {
                hash: TargetHash::new("413a2991febb853a43ce1a7a56f10225"),
                labels: Labels::new(&["my_label"]),
                oncall: Some(Oncall::new("my_team")),
                ..BuckTarget::testing("test2", "fbcode//me", "prelude//rules.bzl:cxx_library")
            }),
            TargetsEntry::Error(BuckError {
                package: Package::new("fbcode//broken"),
                error: "Broken :(".to_owned(),
            }),
        ];
        assert_eq!(res.0.len(), required.len());
        for x in &required {
            assert!(res.0.contains(x));
        }

        let mut file = NamedTempFile::new().unwrap();
        json::write_json_lines(file.as_file_mut(), &required).unwrap();
        let output: Vec<TargetsEntry> = json::read_file_lines(file.path()).unwrap();
        assert_eq!(output, required);
    }

    #[test]
    fn test_read_targets_extra() {
        // Check we don't choke if Buck2 suddenly starts emitting extra fields in targets, or
        // someone doesn't trim as tightly as they could.
        let value = serde_json::json!(
            [
                {
                    "buck.type": "prelude//rules.bzl:python_library",
                    "buck.deps": ["toolchains//:python", "fbcode//python:library"],
                    "buck.inputs": ["fbcode//me/file.bzl"],
                    "buck.target_hash": "43ce1a7a56f10225413a2991febb853a",
                    "buck.package": "fbcode//me",
                    "buck.random": "new_field",
                    "buck.package_values": {"citadel.labels": ["ci:@fbcode//mode/opt"], "extra": "value"},
                    "name": "test",
                },
            ]
        );
        let file = write_buck_input(value);

        let res = Targets::from_file(file.path()).unwrap();
        assert_eq!(res.0.len(), 1);
    }
}
