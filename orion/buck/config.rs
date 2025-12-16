/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Configuration that we hardcode, because parsing it is too expensive.

use crate::types::CellPath;

/// Certain bzl files should be excluded from transitive impact tracing.
/// This list should *only* be extended if we are certain that changes to the file
/// will have its impact traced through other means, e.g., attribute changes.
pub fn should_exclude_bzl_file_from_transitive_impact_tracing(path: &str) -> bool {
    path.ends_with(".bzl")
        && [
            "fbsource//tools/target_determinator/macros",
            // This is the public interface and auto-imported by all bzl files.
            // By tracing through this, all macros and rules outside the prelude
            // end up being affected, which massively overbuilds on otherwise
            // limited rule changes.
            "prelude//prelude.bzl",
        ]
        .iter()
        .any(|prefix| path.starts_with(*prefix))
}

pub fn is_buck_deployment(path: &CellPath) -> bool {
    path.as_str().starts_with("fbsource//tools/buck2-versions/")
}

// Touching these files will let btd treat everything as affected.
pub fn is_buckconfig_change(path: &CellPath) -> bool {
    // List of important modefile dirs
    const MODE_DIRECTORIES: &[&str] = &[
        "fbsource//arvr/mode/",
        "fbsource//fbandroid/mode/",
        "fbsource//fbcode/mode/",
        "fbsource//fbobjc/mode/",
        "fbsource//xplat/mode/",
    ];

    let ext = path.extension();
    let str = path.as_str();

    // Don't treat changes to buck2 tests as buckconfig changes, otherwise we run way too much CI on
    // these
    if str.contains("buck2/tests") {
        return false;
    }

    ext == Some("buckconfig")
        || str.starts_with("fbsource//tools/buckconfigs/")
        || (ext.is_none()
            && MODE_DIRECTORIES
                .iter()
                .any(|&prefix| str.starts_with(prefix)))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_is_buck_deployment() {
        assert!(is_buck_deployment(&CellPath::new(
            "fbsource//tools/buck2-versions/previous"
        )));
        assert!(is_buck_deployment(&CellPath::new(
            "fbsource//tools/buck2-versions/stable"
        )));
    }

    #[test]
    fn test_is_buckconfig_change() {
        // random buckconfigs
        assert!(!is_buckconfig_change(&CellPath::new(
            "fbcode//some_config.bcfg"
        )));
        // buckconfigs
        assert!(is_buckconfig_change(&CellPath::new(
            "fbsource//.buckconfig"
        )));
        // bcfg
        assert!(is_buckconfig_change(&CellPath::new(
            "fbsource//tools/buckconfigs/abc/xyz.bcfg"
        )));
        assert!(!is_buckconfig_change(&CellPath::new(
            "fbcode//buck2/TARGETS"
        )));
        assert!(!is_buckconfig_change(&CellPath::new(
            "fbcode//buck2/src/file.rs"
        )));
        assert!(is_buckconfig_change(&CellPath::new(
            "fbsource//tools/buckconfigs/cxx/windows/clang.inc"
        )));
        assert!(is_buckconfig_change(&CellPath::new(
            "fbsource//arvr/mode/dv/dev.buckconfig"
        )));
        assert!(is_buckconfig_change(&CellPath::new(
            "fbsource//tools/buckconfigs/fbsource-specific.bcfg"
        )));
        assert!(is_buckconfig_change(&CellPath::new(
            "fbsource//.buckconfig"
        )));
        assert!(!is_buckconfig_change(&CellPath::new(
            "fbcode//buck2/tests/foo_data/.buckconfig"
        )));
    }
}
