/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use crate::ExitStatusExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Context as _;
use audit::audit_cell_arguments;
use audit::audit_config_arguments;
use td_util::command::create_at_file_arg;
use td_util::command::with_command;
use tracing::info;

use crate::cells::CellInfo;
use crate::types::Package;
use crate::types::ProjectRelativePath;
use crate::types::TargetPattern;

/// A struct to represent running Buck2 commands.
/// All methods are `&mut` to avoid simultaneous Buck2 commands.
pub struct Buck2 {
    /// The program to invoke, normally `buck2`.
    program: String,
    /// The result of running `root`, if we have done so yet.
    root: Option<PathBuf>,
    /// The isolation directory to always use when invoking buck
    isolation_dir: Option<String>,
}

impl Buck2 {
    pub fn new(program: String, root: PathBuf) -> Self {
        Self {
            program,
            root: Some(root),
            isolation_dir: None,
        }
    }

    pub fn with_root(program: String, root: PathBuf) -> Self {
        Self {
            program,
            root: Some(root),
            isolation_dir: None,
        }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        match &self.isolation_dir {
            None => {}
            Some(isolation_dir) => {
                command.args(["--isolation-dir", isolation_dir]);
            }
        }
        command
    }

    pub fn root(&mut self) -> anyhow::Result<PathBuf> {
        Ok(self.root.clone().expect("buck root unset"))
    }

    pub fn cells(&mut self) -> anyhow::Result<String> {
        let mut command = self.command();
        command.args(audit_cell_arguments());
        command.current_dir(self.root()?);
        let res = with_command(command, |mut command| {
            let res = command.output()?;
            res.status.exit_result().with_context(|| {
                format!("Buck2 stderr: {}", String::from_utf8_lossy(&res.stderr))
            })?;
            Ok(res)
        })?;
        Ok(String::from_utf8(res.stdout)?)
    }

    pub fn audit_config(&mut self) -> anyhow::Result<String> {
        let mut command = self.command();
        command.args(audit_config_arguments());
        command.current_dir(self.root()?);
        let res = with_command(command, |mut command| {
            let res = command.output()?;
            res.status.exit_result().with_context(|| {
                format!("Buck2 stderr: {}", String::from_utf8_lossy(&res.stderr))
            })?;
            Ok(res)
        })?;
        Ok(String::from_utf8(res.stdout)?)
    }

    /// Does a package exist. Doesn't actually invoke Buck2, but does look at the file system.
    pub fn does_package_exist(&mut self, cells: &CellInfo, x: &Package) -> anyhow::Result<bool> {
        let root = self.root()?;
        for build_file in cells.build_files(&x.cell())? {
            let cell_path = x.join_path(build_file);
            if !cells.is_ignored(&cell_path)
                && root.join(cells.resolve(&cell_path)?.as_str()).exists()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn targets(
        &mut self,
        extra_args: &[String],
        targets: &[TargetPattern],
        output: &Path,
    ) -> anyhow::Result<()> {
        assert!(!targets.is_empty());

        let (_file, at_file) = create_at_file_arg(targets, "\n")?;

        let mut command = self.command();
        command
            .args(targets_arguments())
            .arg("--output")
            .arg(output)
            .arg(at_file)
            .args(extra_args);

        with_command(command, |mut command| {
            Ok(command.status()?.exit_result()?)
        })
    }

    pub fn owners(
        &mut self,
        extra_args: &[String],
        changes: &[ProjectRelativePath],
    ) -> anyhow::Result<String> {
        assert!(!changes.is_empty());

        let (_file, at_file) = create_at_file_arg(changes, "\n")?;

        let mut command = self.command();
        command
            .arg("uquery")
            .arg("--json")
            .arg("owner(\"%s\")")
            .arg(at_file)
            .args(extra_args);
        command.current_dir(self.root()?);

        info!("Running owners query: {:?}", command);

        let res = with_command(command, |mut command| {
            let res = command.output()?;
            res.status.exit_result().with_context(|| {
                format!("Buck2 stderr: {}", String::from_utf8_lossy(&res.stderr))
            })?;
            Ok(res)
        })?;
        Ok(String::from_utf8(res.stdout)?)
    }
}

pub fn targets_arguments() -> &'static [&'static str] {
    &[
        "targets",
        "//...",
        "--streaming",
        "--keep-going",
        "--no-cache",
        "--show-unconfigured-target-hash",
        "--json-lines",
        "--output-attribute=^buck\\.|^name$|^labels$|^ci_srcs$|^ci_srcs_must_match$|^ci_deps$|^remote_execution$",
        "--imports",
        "--package-values-regex=^citadel\\.labels$|^test_config_unification\\.rollout$",
    ]
}
