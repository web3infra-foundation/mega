/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

// #![feature(exit_status_error)]
#![forbid(unsafe_code)]

use std::io;
use std::process::ExitStatus;

// defining own ExitStatus to avoid building under nightly version
pub trait ExitStatusExt {
    fn exit_result(&self) -> Result<(), io::Error>;
}

impl ExitStatusExt for ExitStatus {
    fn exit_result(&self) -> Result<(), io::Error> {
        if self.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "process exited unsuccessfully: {}",
                self
            )))
        }
    }
}

pub mod cells;
pub mod config;
pub mod glob;
pub mod ignore_set;
pub mod labels;
pub mod owners;
pub mod package_resolver;
pub mod run;
pub mod target_graph;
pub mod target_map;
pub mod targets;
pub mod types;
