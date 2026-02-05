/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Equivalent to the Buck2 `glob` to the greatest extent possible.

use glob::{MatchOptions, Pattern};
use itertools::{Either, Itertools};

use crate::types::{Glob, GlobInclusion};
use api_model::buck2::types::ProjectRelativePath;

pub struct GlobSpec {
    include: GlobSet,
    exclude: GlobSet,
}

pub struct GlobSet(Vec<Pattern>);

impl GlobSet {
    pub fn new(globs: &[&str]) -> Self {
        Self(globs.iter().flat_map(|x| Pattern::new(x)).collect())
    }

    pub fn matches(&self, path: &ProjectRelativePath) -> bool {
        let options = MatchOptions {
            require_literal_separator: true,
            require_literal_leading_dot: true,
            // Buck2 is currently case insensitive, but they want to fix that, so we should be more picky
            case_sensitive: true,
        };
        self.0
            .iter()
            .any(|x| x.matches_with(path.as_str(), options))
    }
}

impl GlobSpec {
    pub fn new(globs: &[Glob]) -> Self {
        // We just throw away any inaccurate globs for now, and rely on the macro layer spotting them.
        // We probably want a lint pass sooner or later.
        let (include, exclude): (Vec<_>, Vec<_>) =
            globs.iter().partition_map(|x| match x.unpack() {
                (GlobInclusion::Include, x) => Either::Left(x),
                (GlobInclusion::Exclude, x) => Either::Right(x),
            });

        Self {
            include: GlobSet::new(&include),
            exclude: GlobSet::new(&exclude),
        }
    }

    pub fn matches(&self, path: &ProjectRelativePath) -> bool {
        self.include.matches(path) && !self.exclude.matches(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn many(globs: &[&str], path: &str, res: bool) {
        assert_eq!(
            res,
            GlobSpec::new(&globs.iter().map(|x| Glob::new(x)).collect::<Vec<_>>())
                .matches(&ProjectRelativePath::new(path)),
            "With {globs:?} and {path:?}"
        )
    }

    fn one(glob: &str, path: &str, res: bool) {
        many(&[glob], path, res);
    }

    #[test]
    fn test_glob() {
        one("abc*", "abcxyz", true);
        one("abc*", "abcxyz/bar", false);
        one("foo/*", "foo/abc", true);
        one("foo/*", "foo/abc/bar", false);
        one("**/*.java", "foo/bar/baz/me.java", true);
        one("**/*.java", "foo/bar/baz/me.jar", false);
        one("simple", "simple", true);
        one("foo/bar/**", "foo/bar/baz/qux.txt", true);
        one("foo/bar/**", "foo/bar/magic", true);
        one("foo/bar/**", "foo/bard", false);
        one("foo/bar/**", "elsewhere", false);
    }

    #[test]
    fn test_glob_negation() {
        many(
            &["foo/bar/**", "!foo/bar/baz/**"],
            "foo/bar/hello/file.txt",
            true,
        );
        many(
            &["foo/bar/**", "!foo/bar/baz/**"],
            "foo/bar/baz/file.txt",
            false,
        );
        many(
            &["!foo/bar/baz/**", "foo/bar/**"],
            "foo/bar/baz/file.txt",
            false,
        );
    }

    #[test]
    fn test_dot_handling() {
        // require_literal_leading_dot is true, so this fails
        // This test is to document the behavior - don't take
        // it as an endorsement
        many(
            &["www/**/*", "www/**/.*"],
            "www/.llms/rules/mvwa_integrity_config.md",
            false,
        );
        // Globs that cover literally everything in wwww
        many(
            &["www/**/*", "www/**/.*/**/*", "www/**/.*", "www/**/.*/**/.*"],
            "www/.llms/rules/mvwa_integrity_config.md",
            true,
        );
        many(
            &["www/**/*", "www/**/.*/**/*", "www/**/.*", "www/**/.*/**/.*"],
            "www/foo/bar/.llms/rules/mvwa_integrity_config.md",
            true,
        );
        many(
            &["www/**/*", "www/**/.*/**/*", "www/**/.*", "www/**/.*/**/.*"],
            "www/foo/bar/.llms/rules/.mvwa_integrity_config.md",
            true,
        );
        // Proof that all are needed
        many(
            &["www/**/.*/**/*", "www/**/.*", "www/**/.*/**/.*"],
            "www/foo/bar/llms/rules/mvwa_integrity_config.md",
            false,
        );
        many(
            &["www/**/*", "www/**/.*", "www/**/.*/**/.*"],
            "www/foo/bar/.llms/rules/mvwa_integrity_config.md",
            false,
        );
        many(
            &["www/**/*", "www/**/.*/**/*", "www/**/.*/**/.*"],
            "www/foo/bar/llms/rules/.mvwa_integrity_config.md",
            false,
        );
        many(
            &["www/**/*", "www/**/.*/**/*", "www/**/.*"],
            "www/foo/bar/.llms/rules/.mvwa_integrity_config.md",
            false,
        );
    }
}
