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
    ffi::OsString,
    io::Write,
    process::{ChildStdout, Command, Stdio},
    time::Instant,
};

use anyhow::anyhow;
use itertools::Itertools;
use tempfile::NamedTempFile;
use tracing::debug;

use crate::workflow_error::WorkflowError;

/// Run a command printing out debugging information.
pub fn with_command<T>(
    command: Command,
    run: impl Fn(Command) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    debug!("Running: {}", display_command(&command));
    let start = Instant::now();
    let res = run(command)?;
    debug!("Command succeeded in {:.2}s", start.elapsed().as_secs_f64());
    Ok(res)
}

/// Works only for command lines we produce, without environment variables
/// or any argument escaping.
pub fn display_command(command: &Command) -> String {
    let mut res = command.get_program().to_owned();
    for x in command.get_args() {
        res.push(" ");
        res.push(x);
    }
    res.to_string_lossy().into_owned()
}

/// Spawns a command with stdout piped and returns the child process and stdout handle.
pub fn spawn(mut command: Command) -> Result<(std::process::Child, ChildStdout), WorkflowError> {
    command.stdout(Stdio::piped());
    let mut child = command.spawn().map_err(|err| anyhow!(err))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Failed to capture stdout"))?;
    Ok((child, stdout))
}

/// Creates a temporary file with the given content and returns an OsString
/// in "@file" format suitable for command line tools that accept file arguments.
///
/// This is commonly used with tools like Buck2 that accept target lists via @file syntax.
pub fn create_at_file_arg<T: AsRef<str>>(
    items: &[T],
    separator: &str,
) -> anyhow::Result<(NamedTempFile, OsString)> {
    let mut file = NamedTempFile::new()?;
    let content = items.iter().map(|x| x.as_ref()).join(separator);
    file.write_all(content.as_bytes())?;
    file.flush()?;

    let mut at_file = OsString::new();
    at_file.push("@");
    at_file.push(file.path());

    Ok((file, at_file))
}
