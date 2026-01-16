/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! The projects where a verifiable has come from.
//! We should seek to minimize (eventually remove) any project differences.

#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt as _;
use std::{cmp::Eq, hash::Hash, io, path::PathBuf, process::Command};

use clap::ValueEnum;
use parse_display::Display;

#[derive(ValueEnum, Debug, Display, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[display(style = "lowercase")]
pub enum TdProject {
    Configerator,
    Fbcode,
    Fbandroid,
    Fbobjc,
    Genai,
    Mobile,
    RL,
    Wacommon,
    Waclient,
    Waios,
    Waserver,
    Www,
    Xplat,
    Fasttrack,
}

impl TdProject {
    pub fn is_mobile(&self) -> bool {
        matches!(self, Self::Fbandroid | Self::Fbobjc)
    }
}

pub fn get_repo_root() -> io::Result<PathBuf> {
    let mut output = Command::new("hg").arg("root").output()?;
    output.stdout.truncate(output.stdout.trim_ascii_end().len());

    #[cfg(unix)]
    let s = OsString::from_vec(output.stdout);
    #[cfg(windows)]
    let s = String::from_utf8(output.stdout).map_err(io::Error::other)?;

    Ok(PathBuf::from(s))
}
