/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! All these types mirror their equivalent in the Buck2 codebase

use std::ffi::OsStr;
use std::fmt;
use std::hash::Hash;
use std::str::FromStr;

use parse_display::Display;
use serde::Deserialize;
use serde::Serialize;
use td_util::string::InternString;

use crate::cells::CellInfo;
use crate::labels::Labels;
use utoipa::ToSchema;

/// Example: `fbcode//buck2:buck2`
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    Display,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord
)]
pub struct TargetLabel(InternString);

impl TargetLabel {
    pub fn new(target: &str) -> Self {
        Self(InternString::new(target))
    }

    fn split(&self) -> (&str, &str) {
        self.0.as_str().rsplit_once(':').unwrap()
    }

    /// ```
    /// use td_util_buck::types::Package;
    /// use td_util_buck::types::TargetLabel;
    /// assert_eq!(
    ///     TargetLabel::new("foo//bar/baz:qux").package(),
    ///     Package::new("foo//bar/baz")
    /// );
    /// ```
    pub fn package(&self) -> Package {
        Package::new(self.split().0)
    }

    /// ```
    /// use td_util_buck::types::TargetLabel;
    /// use td_util_buck::types::TargetName;
    /// assert_eq!(
    ///     TargetLabel::new("foo//bar/baz:qux").target_name(),
    ///     TargetName::new("qux")
    /// );
    /// ```
    pub fn target_name(&self) -> TargetName {
        TargetName::new(self.split().1)
    }

    pub fn key(&self) -> TargetLabelKey {
        let (pkg, name) = self.split();
        TargetLabelKey(Package::new(pkg), TargetName::new(name))
    }

    /// ```
    /// use td_util_buck::types::TargetLabel;
    /// assert!(!TargetLabel::new("foo//bar/baz:qux").is_package_relative());
    /// assert!(TargetLabel::new(":qux").is_package_relative());
    /// ```
    pub fn is_package_relative(&self) -> bool {
        self.0.as_str().starts_with(':')
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<OsStr> for TargetLabel {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(self.as_str())
    }
}

/// Equivalent to a `TargetLabel`, used to identify a label efficiently,
/// including when produced by the `buck2 targets` JSON output.
pub struct TargetLabelKey(Package, TargetName);

impl TargetLabelKey {
    pub fn to_ref(&self) -> TargetLabelKeyRef<'_> {
        TargetLabelKeyRef::new(&self.0, &self.1)
    }
}

/// A reference to the parts of a `TargetLabelKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetLabelKeyRef<'a>(&'a Package, &'a TargetName);

impl<'a> TargetLabelKeyRef<'a> {
    pub fn new(package: &'a Package, target_name: &'a TargetName) -> Self {
        Self(package, target_name)
    }
}

/// Example: `fbcode//buck2:` or `fbcode//buck2/...`
#[derive(Debug, Clone, Hash, PartialEq, Eq, Display, Deserialize, Serialize)]
pub struct TargetPattern(String);

impl TargetPattern {
    pub fn new(target: &str) -> Self {
        Self(target.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// ```
    /// use td_util_buck::types::TargetPattern;
    /// assert!(!TargetPattern::new("foo//bar/...").is_specific_target());
    /// assert!(!TargetPattern::new("foo//bar/baz:").is_specific_target());
    /// assert!(TargetPattern::new("foo//bar:baz").is_specific_target());
    /// ```
    pub fn is_specific_target(&self) -> bool {
        match self.0.rsplit_once(':') {
            None => false,
            Some((_, target)) => !target.is_empty(),
        }
    }

    /// Convert to a `TargetLabel` if the pattern represents a specific target.
    pub fn as_target_label(&self) -> Option<TargetLabel> {
        if self.is_specific_target() {
            Some(TargetLabel::new(self.as_str()))
        } else {
            None
        }
    }

    /// Convert a pattern representing a single package into that `Package`.
    /// These are the patterns that end in the `:` suffix.
    ///
    /// ```
    /// use td_util_buck::types::Package;
    /// use td_util_buck::types::TargetPattern;
    /// assert_eq!(
    ///     TargetPattern::new("foo//:").as_package_pattern(),
    ///     Some(Package::new("foo//"))
    /// );
    /// assert_eq!(
    ///     TargetPattern::new("foo//bar:").as_package_pattern(),
    ///     Some(Package::new("foo//bar"))
    /// );
    /// assert_eq!(
    ///     TargetPattern::new("foo//bar/baz:").as_package_pattern(),
    ///     Some(Package::new("foo//bar/baz"))
    /// );
    /// assert_eq!(TargetPattern::new("foo//...").as_package_pattern(), None);
    /// assert_eq!(TargetPattern::new("foo//bar").as_package_pattern(), None);
    /// ```
    pub fn as_package_pattern(&self) -> Option<Package> {
        self.as_str().strip_suffix(':').map(Package::new)
    }

    /// Convert a pattern representing a recursive package into that `Package`.
    /// These are the patterns that end in the `...` suffix.
    ///
    /// ```
    /// use td_util_buck::types::Package;
    /// use td_util_buck::types::TargetPattern;
    /// assert_eq!(
    ///     TargetPattern::new("foo//...").as_recursive_pattern(),
    ///     Some(Package::new("foo//"))
    /// );
    /// assert_eq!(
    ///     TargetPattern::new("foo//bar/...").as_recursive_pattern(),
    ///     Some(Package::new("foo//bar"))
    /// );
    /// assert_eq!(
    ///     TargetPattern::new("foo//bar/baz/...").as_recursive_pattern(),
    ///     Some(Package::new("foo//bar/baz"))
    /// );
    /// assert_eq!(TargetPattern::new("foo//bar:").as_recursive_pattern(), None);
    /// assert_eq!(TargetPattern::new("foo//bar").as_recursive_pattern(), None);
    /// ```
    pub fn as_recursive_pattern(&self) -> Option<Package> {
        let prefix = self.as_str().strip_suffix("...")?;
        // if it is foo// we need to keep the slash, if foo//bar/ we need to remove it
        Some(Package::new(match prefix.strip_suffix('/') {
            Some(x) if !x.ends_with('/') => x,
            _ => prefix,
        }))
    }

    /// ```
    /// use td_util_buck::types::TargetLabel;
    /// use td_util_buck::types::TargetPattern;
    /// assert!(TargetPattern::new("foo//bar/baz:").matches(&TargetLabel::new("foo//bar/baz:qux")));
    /// assert!(
    ///     !TargetPattern::new("foo//bar/baz:").matches(&TargetLabel::new("foo//bar/baz/boo:qux")),
    /// );
    /// assert!(!TargetPattern::new("foo//bar/baz:").matches(&TargetLabel::new("foo//bar:qux")));
    /// assert!(TargetPattern::new("foo//:").matches(&TargetLabel::new("foo//:qux")));
    /// assert!(!TargetPattern::new("foo//:").matches(&TargetLabel::new("foo//bar:qux")));
    /// assert!(TargetPattern::new("foo//...").matches(&TargetLabel::new("foo//bar/baz:qux")));
    /// assert!(TargetPattern::new("foo//...").matches(&TargetLabel::new("foo//baz:qux")));
    /// assert!(TargetPattern::new("foo//...").matches(&TargetLabel::new("foo//:qux")));
    /// assert!(TargetPattern::new("foo//bar/...").matches(&TargetLabel::new("foo//bar:qux")));
    /// assert!(TargetPattern::new("foo//bar/...").matches(&TargetLabel::new("foo//bar/baz:qux")));
    /// assert!(
    ///     !TargetPattern::new("foo//bar/...").matches(&TargetLabel::new("foo//bard/baz:qux")),
    /// );
    /// assert!(
    ///     !TargetPattern::new("foo//bar/...").matches(&TargetLabel::new("foo//moo/bar/baz:qux")),
    /// );
    /// assert!(
    ///     TargetPattern::new("foo//bar/a:literal").matches(&TargetLabel::new("foo//bar/a:literal")),
    /// );
    /// assert!(
    ///     !TargetPattern::new("foo//bar/a:literal").matches(&TargetLabel::new("foo//bar/a:nother")),
    /// );
    /// ```
    pub fn matches(&self, target: &TargetLabel) -> bool {
        self.matches_str(target.as_str())
    }

    /// Like `matches` but takes a string instead of a `TargetLabel`.
    /// The string is required to be a valid target label.
    pub fn matches_str(&self, target: &str) -> bool {
        if self.0.ends_with(':') {
            // You can only have a name after a :, so if you match to the colon, you must be good
            target.starts_with(self.0.as_str())
        } else if let Some(prefix) = self.0.strip_suffix("/...") {
            match target.strip_prefix(prefix) {
                Some(rest) => rest.starts_with(':') || rest.starts_with('/'),
                None => false,
            }
        } else {
            self.0.as_str() == target
        }
    }

    /// ```
    /// use td_util_buck::types::Package;
    /// use td_util_buck::types::TargetPattern;
    /// assert!(TargetPattern::new("foo//bar:").matches_package(&Package::new("foo//bar")));
    /// assert!(!TargetPattern::new("foo//bar:").matches_package(&Package::new("foo//bard")));
    /// assert!(!TargetPattern::new("foo//bard:").matches_package(&Package::new("foo//bar")));
    /// assert!(!TargetPattern::new("foo//bar:").matches_package(&Package::new("foo//baz")));
    /// assert!(!TargetPattern::new("foo//baz:").matches_package(&Package::new("foo//bar")));
    /// assert!(!TargetPattern::new("foo//bar:").matches_package(&Package::new("foo//bar/baz")));
    /// assert!(TargetPattern::new("foo//bar/...").matches_package(&Package::new("foo//bar")));
    /// assert!(TargetPattern::new("foo//bar/...").matches_package(&Package::new("foo//bar/baz")));
    /// assert!(!TargetPattern::new("foo//bar/...").matches_package(&Package::new("foo//bard")));
    /// assert!(!TargetPattern::new("foo//bar/...").matches_package(&Package::new("foo//baz")));
    /// assert!(TargetPattern::new("foo//...").matches_package(&Package::new("foo//baz")));
    /// assert!(TargetPattern::new("foo//...").matches_package(&Package::new("foo//")));
    /// ```
    pub fn matches_package(&self, package: &Package) -> bool {
        let pkg = package.as_str();
        if let Some(prefix) = self.0.strip_suffix(':') {
            prefix == pkg
        } else if let Some(prefix) = self.0.strip_suffix("/...") {
            match pkg.strip_prefix(prefix) {
                Some(rest) => rest.is_empty() || rest.starts_with('/'),
                None => false,
            }
        } else {
            false
        }
    }
}

impl AsRef<str> for TargetPattern {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for TargetPattern {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

/// Example: `buck2` bit in `fbcode//build:buck2`
#[derive(
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    Clone
)]
pub struct TargetName(InternString);

impl TargetName {
    pub fn new(name: &str) -> Self {
        Self(InternString::new(name))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Example: `fbcode` in `fbcode//buck2:buck2`
#[derive(Clone, Debug, Hash, PartialEq, Eq, Display)]
pub struct CellName(String);

impl CellName {
    pub fn new(cell: &str) -> Self {
        Self(cell.to_owned())
    }

    pub fn join(&self, path: &CellRelativePath) -> CellPath {
        CellPath(InternString::from_string(format!("{}//{}", self.0, path.0)))
    }

    pub fn as_cell_path(&self) -> CellPath {
        self.join(&CellRelativePath::new(""))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Example: `fbcode//buck2/TARGETS`
#[derive(Debug, Hash, PartialEq, Eq, Deserialize, Serialize, Clone, Display)]
pub struct CellPath(InternString);

impl CellPath {
    pub fn new(path: &str) -> Self {
        assert!(
            path.contains("//"),
            "Invalid CellPath, missing `//` from `{path}`"
        );
        Self(InternString::new(path))
    }

    pub fn cell(&self) -> CellName {
        CellName(self.0.as_str().split_once("//").unwrap().0.to_owned())
    }

    pub fn path(&self) -> CellRelativePath {
        CellRelativePath(self.0.as_str().split_once("//").unwrap().1.to_owned())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// ```
    /// use td_util_buck::types::CellPath;
    /// assert_eq!(
    ///     CellPath::new("foo//bar.bzl").parent(),
    ///     CellPath::new("foo//")
    /// );
    /// assert_eq!(
    ///     CellPath::new("foo//bar.bzl/baz").parent(),
    ///     CellPath::new("foo//bar.bzl")
    /// );
    /// ```
    pub fn parent(&self) -> CellPath {
        let p = self.path();
        let p_parent = p.parent();
        if let Some(x) = p_parent {
            Self(InternString::from_string(format!(
                "{}//{}",
                self.cell().as_str(),
                x.as_str()
            )))
        } else {
            Self(InternString::from_string(format!(
                "{}//",
                self.cell().as_str()
            )))
        }
    }

    /// Convert a `CellPath` into an identically valued `Package`.
    /// Only makes sense for directories that you know must be on package boundaries,
    /// e.g. `BUCK` or `PACKAGE` files.
    pub fn as_package(&self) -> Package {
        Package(self.0.clone())
    }

    /// ```
    /// use td_util_buck::types::CellPath;
    /// assert_eq!(CellPath::new("foo//bar.bzl").extension(), Some("bzl"));
    /// assert_eq!(CellPath::new("foo//bar.bzl/baz").extension(), None);
    /// assert_eq!(CellPath::new("foo//bar/baz").extension(), None);
    /// ```
    pub fn extension(&self) -> Option<&str> {
        self.0
            .as_str()
            .rsplit_once('/')
            .unwrap_or_default()
            .1
            .rsplit_once('.')
            .map(|x| x.1)
    }

    /// ```
    /// use td_util_buck::cells::CellInfo;
    /// use td_util_buck::types::CellPath;
    /// let cells = CellInfo::testing();
    /// assert_eq!(
    ///     CellPath::new("foo//bar/source.txt")
    ///         .is_target_file(&cells)
    ///         .unwrap(),
    ///     false
    /// );
    /// assert_eq!(
    ///     CellPath::new("foo//bar/BUCK")
    ///         .is_target_file(&cells)
    ///         .unwrap(),
    ///     true
    /// );
    /// assert_eq!(
    ///     CellPath::new("foo//bar/BUCK.v2")
    ///         .is_target_file(&cells)
    ///         .unwrap(),
    ///     true
    /// );
    /// assert_eq!(
    ///     CellPath::new("foo//bar/NOT_BUCK")
    ///         .is_target_file(&cells)
    ///         .unwrap(),
    ///     false
    /// );
    /// assert_eq!(
    ///     CellPath::new("foo//bar/TARGETS")
    ///         .is_target_file(&cells)
    ///         .unwrap(),
    ///     false
    /// );
    /// assert_eq!(
    ///     CellPath::new("foo//BUCK").is_target_file(&cells).unwrap(),
    ///     true
    /// );
    /// assert_eq!(
    ///     CellPath::new("fbcode//TARGETS")
    ///         .is_target_file(&cells)
    ///         .unwrap(),
    ///     true
    /// );
    /// assert_eq!(
    ///     CellPath::new("prelude//apple/TARGETS.v2")
    ///         .is_target_file(&cells)
    ///         .unwrap(),
    ///     true
    /// );
    /// ```
    pub fn is_target_file(&self, cells: &CellInfo) -> anyhow::Result<bool> {
        let contents = self.0.as_str();
        for build_file in cells.build_files(&self.cell())? {
            if let Some(suffix) = contents.strip_suffix(build_file) {
                if suffix.ends_with('/') {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// ```
    /// use td_util_buck::types::CellPath;
    /// assert!(!CellPath::new("foo//bar/source.txt").is_package_file());
    /// assert!(CellPath::new("foo//bar/PACKAGE").is_package_file());
    /// assert!(!CellPath::new("foo//bar/PACKAGE.v2").is_package_file());
    /// assert!(!CellPath::new("foo//bar/NOT_PACKAGE").is_package_file());
    /// assert!(!CellPath::new("foo//bar/TARGETS").is_package_file());
    /// assert!(CellPath::new("foo//PACKAGE").is_package_file());
    /// assert!(CellPath::new("foo//bar/BUCK_TREE").is_package_file());
    /// ```
    pub fn is_package_file(&self) -> bool {
        let s = self.0.as_str();
        s.ends_with("/PACKAGE") || s.ends_with("/BUCK_TREE")
    }

    /// ```
    /// use td_util_buck::types::CellPath;
    /// assert!(!CellPath::new("foo//bar/rule.bzl").is_prelude_bzl_file());
    /// assert!(!CellPath::new("prelude//apple/TARGETS.v2").is_prelude_bzl_file());
    /// assert!(CellPath::new("prelude//apple/rule.bzl").is_prelude_bzl_file());
    /// ```
    pub fn is_prelude_bzl_file(&self) -> bool {
        let contents = self.0.as_str();
        contents.starts_with("prelude//") && contents.ends_with(".bzl")
    }
}

/// Example: `fbcode//buck2`
#[derive(
    Clone,
    Debug,
    Display,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize
)]
pub struct Package(InternString);

impl Package {
    pub fn new(package: &str) -> Self {
        Self(InternString::new(package))
    }

    pub fn join(&self, name: &TargetName) -> TargetLabel {
        TargetLabel(InternString::new3(self.0.as_str(), ":", name.0.as_str()))
    }

    pub fn join_path(&self, path: &str) -> CellPath {
        CellPath(InternString::new3(self.0.as_str(), "/", path))
    }

    pub fn cell(&self) -> CellName {
        CellName(self.0.as_str().split_once("//").unwrap().0.to_owned())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_pattern(&self) -> TargetPattern {
        TargetPattern::new(&format!("{}:", self.0))
    }

    /// Represents the directory in which this package lives
    pub fn as_cell_path(&self) -> CellPath {
        CellPath(self.0.clone())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageValues {
    #[serde(rename = "citadel.labels", default)]
    pub labels: Labels,
}

impl PackageValues {
    pub fn new(values: &[&str]) -> Self {
        Self {
            labels: Labels::new(values),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
}

/// Example: `prelude//rules.bzl:genrule`
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct RuleType(TargetLabel);

impl RuleType {
    pub fn new(rule: &str) -> Self {
        Self(TargetLabel::new(rule))
    }

    /// ```
    /// use td_util_buck::types::RuleType;
    /// assert_eq!(
    ///     RuleType::new("prelude//rules.bzl:genrule").short(),
    ///     "genrule"
    /// );
    /// ```
    pub fn short(&self) -> &str {
        let contents = self.0.0.as_str();
        match contents.rsplit_once(':') {
            None => contents,
            Some((_, x)) => x,
        }
    }

    /// ```
    /// use td_util_buck::types::CellPath;
    /// use td_util_buck::types::RuleType;
    /// assert_eq!(
    ///     RuleType::new("prelude//rules.bzl:genrule").file(),
    ///     CellPath::new("prelude//rules.bzl")
    /// );
    /// ```
    pub fn file(&self) -> CellPath {
        let contents = self.0.0.as_str();
        match contents.rsplit_once(':') {
            None => CellPath::new(contents),
            Some((x, _)) => CellPath::new(x),
        }
    }
}

/// Example: `ci_efficiency`
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct Oncall(InternString);

impl Oncall {
    pub fn new(oncall: &str) -> Self {
        Self(InternString::new(oncall))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for Oncall {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct TargetHash(String);

impl TargetHash {
    pub fn new(hash: &str) -> Self {
        Self(hash.to_owned())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Display, Deserialize, Serialize, ToSchema)]
pub struct ProjectRelativePath(String);

impl ProjectRelativePath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn join(&self, suffix: &str) -> Self {
        if self.0.is_empty() {
            Self(suffix.to_owned())
        } else {
            Self(format!("{}/{}", self.0, suffix))
        }
    }

    /// ```
    /// use td_util_buck::types::ProjectRelativePath;
    /// assert_eq!(
    ///     ProjectRelativePath::new("foo/bar.bzl").extension(),
    ///     Some("bzl")
    /// );
    /// assert_eq!(
    ///     ProjectRelativePath::new("foo/bar.bzl/baz").extension(),
    ///     None
    /// );
    /// assert_eq!(ProjectRelativePath::new("foo/bar/baz").extension(), None);
    /// ```
    pub fn extension(&self) -> Option<&str> {
        self.0
            .as_str()
            .rsplit_once('/')
            .unwrap_or_default()
            .1
            .rsplit_once('.')
            .map(|x| x.1)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ProjectRelativePath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug)]
pub struct CellRelativePath(String);

impl CellRelativePath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    // returns the parent path in CellRelativePath
    // if the path doesn't have parent return None
    pub fn parent(&self) -> Option<CellRelativePath> {
        let split = self.0.rsplit_once('/');
        if let Some((result1, _result2)) = split {
            let parent_path = CellRelativePath(result1.to_owned());
            Some(parent_path)
        } else {
            None
        }
    }
}

/// Is a [`Glob`] one to include or exclude.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum GlobInclusion {
    Include,
    Exclude,
}

/// Example: `fbcode/buck2/**` or `**/*.java`.
/// May start with `!` to indicate negation.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Display, Deserialize, Serialize)]
pub struct Glob(String);

impl Glob {
    pub fn new(pattern: &str) -> Self {
        Self(pattern.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn unpack(&self) -> (GlobInclusion, &str) {
        let s = self.0.as_str();
        match s.strip_prefix('!') {
            Some(s) => (GlobInclusion::Exclude, s),
            None => (GlobInclusion::Include, s),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PatternType {
    Package,
    Recursive,
}

impl Package {
    pub fn to_target_pattern(&self, pattern_type: PatternType) -> TargetPattern {
        let package_path = self.as_str();

        match pattern_type {
            PatternType::Package => TargetPattern::new(&format!("{}:", package_path)),
            PatternType::Recursive => {
                let is_cell_root = package_path.ends_with("//");
                let recursive_suffix = if is_cell_root { "..." } else { "/..." };
                TargetPattern::new(&format!("{}{}", package_path, recursive_suffix))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let s = "foo//bar:baz";
        let t = TargetLabel::new(s);
        assert_eq!(t.as_str(), s);
        assert_eq!(t.to_string(), s);
    }

    #[test]
    fn test_package_to_target_pattern() {
        let subdirectory = Package::new("fbcode//foo/bar");
        assert_eq!(
            subdirectory
                .to_target_pattern(PatternType::Package)
                .as_str(),
            "fbcode//foo/bar:"
        );
        assert_eq!(
            subdirectory
                .to_target_pattern(PatternType::Recursive)
                .as_str(),
            "fbcode//foo/bar/..."
        );

        let cell_root = Package::new("fbcode//");
        assert_eq!(
            cell_root.to_target_pattern(PatternType::Package).as_str(),
            "fbcode//:"
        );
        assert_eq!(
            cell_root.to_target_pattern(PatternType::Recursive).as_str(),
            "fbcode//..."
        );
    }

    #[test]
    fn test_recursive_pattern_roundtrip() {
        let cell_root_pattern = TargetPattern::new("foo//...");
        let cell_root_package = cell_root_pattern.as_recursive_pattern().unwrap();
        let cell_root_converted = cell_root_package.to_target_pattern(PatternType::Recursive);
        assert_eq!(cell_root_converted.as_str(), "foo//...");

        let subdirectory_pattern = TargetPattern::new("foo//bar/...");
        let subdirectory_package = subdirectory_pattern.as_recursive_pattern().unwrap();
        let subdirectory_converted = subdirectory_package.to_target_pattern(PatternType::Recursive);
        assert_eq!(subdirectory_converted.as_str(), "foo//bar/...");
    }
}
