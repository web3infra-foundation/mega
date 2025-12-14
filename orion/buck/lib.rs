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

// 1. 定义一个扩展 Trait
pub trait ExitStatusExt {
    fn exit_result(&self) -> Result<(), io::Error>;
}

// 2. 为 ExitStatus 实现这个 Trait
impl ExitStatusExt for ExitStatus {
    fn exit_result(&self) -> Result<(), io::Error> {
        if self.success() {
            Ok(())
        } else {
            // 由于标准库的 ExitStatusError 也是 unstable 的，
            // 这里我们需要返回一个 std::io::Error 来替代。
            // 这里的错误信息可以根据你的需要自定义。
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("process exited unsuccessfully: {}", self),
            ))
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
