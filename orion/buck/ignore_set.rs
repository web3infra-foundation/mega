/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Equivalent to the Buck2 `IgnoreSet` type.

use std::sync::LazyLock;

use globset::GlobSetBuilder;
use regex::Regex;

#[derive(Debug, Default)]
pub struct IgnoreSet {
    globset: globset::GlobSet,
}

impl IgnoreSet {
    /// If the `spec` is wrong, Buck2 will fail when run, so leave Buck2 to produce the errors instead.
    pub fn new(spec: &str) -> Self {
        Self::new_result(spec).unwrap_or_default()
    }

    /// Creates an IgnoreSet from an "ignore spec".
    ///
    /// This is modeled after buck1's parsing of project.ignores.
    ///
    /// An ignore spec is a comma-separated list of ignore patterns. If an ignore pattern
    /// contains a glob character, then it uses java.nio.file.FileSystem.getPathMatcher,
    /// otherwise it creates a com.facebook.buck.io.filesystem.RecursivePathMatcher
    ///
    /// Java's path matcher does not allow  '*' to cross directory boundaries. We get
    /// the RecursivePathMatcher behavior by identifying non-globby things and appending
    /// a '/**'.
    ///
    /// We don't follow the implicit ignoring of buck-out, since we don't expect to see
    /// any committed files in buck-out.
    ///
    /// Differences from Buck2:
    ///
    /// In Buck2, each directory along the path is matched, along with the file itself.
    /// In BTD we only match the file itself. To map over this difference, we change
    /// $X to {$X,$X/**} which will trigger the same behavior.
    ///
    /// Buck2 actually does that for literals, even though it doesn't need to.
    pub fn new_result(spec: &str) -> anyhow::Result<Self> {
        let mut patterns_builder = GlobSetBuilder::new();
        for val in spec.split(',') {
            let val = val.trim();
            if val.is_empty() {
                continue;
            }

            let val = val.trim_end_matches('/');

            static GLOB_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[*?{\[]").unwrap());

            if GLOB_CHARS.is_match(val) {
                patterns_builder.add(
                    globset::GlobBuilder::new(&format!("{{{},{}/**}}", val, val))
                        .literal_separator(true)
                        .build()?,
                );
            } else {
                patterns_builder.add(globset::Glob::new(&format!("{{{},{}/**}}", val, val))?);
            }
        }

        Ok(Self {
            globset: patterns_builder.build()?,
        })
    }

    /// Returns whether any pattern matches.
    pub fn is_match(&self, path: &str) -> bool {
        self.globset.is_match(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_set_defaults() {
        let set = IgnoreSet::new("extra, foo/bar, **/*.pyc");
        assert!(set.is_match("foo/bar/bar.txt"));
        assert!(!set.is_match("foo/bar.txt"));
        assert!(set.is_match("extra/bar/baz/foo.txt"));
        assert!(set.is_match("hello/world/file.pyc"));
    }

    #[test]
    fn test_ignore_directory_ignore_files() {
        let set = IgnoreSet::new("foo/, bar/baz/**/tests, qux/*/test");
        assert!(set.is_match("bar/baz/magic/tests/file.c"));
        assert!(!set.is_match("bar/baz/magic/test/file.c"));
        assert!(set.is_match("qux/file/test"));
        assert!(set.is_match("qux/file/test/file.c"));
    }
}
