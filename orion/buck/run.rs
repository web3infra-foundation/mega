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
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Context as _};
use api_model::buck2::types::ProjectRelativePath;
use audit::{audit_cell_arguments, audit_config_arguments};
use td_util::command::{create_at_file_arg, with_command};
use tracing::info;

use crate::{
    cells::CellInfo,
    platform::append_platform_config,
    types::{Package, TargetLabel, TargetPattern},
    ExitStatusExt,
};

/// A struct to represent running Buck2 commands.
/// All methods are `&mut` to avoid simultaneous Buck2 commands.
pub struct Buck2 {
    /// The program to invoke, normally `buck2`.
    program: String,
    /// The result of running `root`, if we have done so yet.
    root: Option<PathBuf>,
}

impl Buck2 {
    pub fn new(program: String, root: PathBuf) -> Self {
        Self {
            program,
            root: Some(root),
        }
    }

    pub fn with_root(program: String, root: PathBuf) -> Self {
        Self {
            program,
            root: Some(root),
        }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .env("BUCKD_STARTUP_TIMEOUT", "30")
            .env("BUCKD_STARTUP_INIT_TIMEOUT", "1200");
        command
    }

    fn kill_daemon(&self) {
        let mut command = self.command();
        command.arg("kill");
        let _ = command.status();
    }

    fn run_output_with_retry<F>(&self, mut make_command: F) -> anyhow::Result<std::process::Output>
    where
        F: FnMut() -> Command,
    {
        const MAX_ATTEMPTS: usize = 2;
        for attempt in 0..MAX_ATTEMPTS {
            let command = make_command();
            let res = with_command(command, |mut command| Ok(command.output()?))?;
            if res.status.success() {
                return Ok(res);
            }

            let stderr = String::from_utf8_lossy(&res.stderr).to_string();
            if attempt + 1 < MAX_ATTEMPTS && should_retry_buck2_daemon(&stderr) {
                tracing::warn!(
                    "buck2 daemon connection failed; retrying after kill (attempt {}/{})",
                    attempt + 1,
                    MAX_ATTEMPTS
                );
                self.kill_daemon();
                continue;
            }

            return Err(anyhow!("Buck2 stderr: {}", stderr));
        }

        Err(anyhow!("Buck2 failed after {} attempts", MAX_ATTEMPTS))
    }

    pub fn root(&mut self) -> anyhow::Result<PathBuf> {
        Ok(self.root.clone().expect("buck root unset"))
    }

    pub fn cells(&mut self) -> anyhow::Result<String> {
        let root = self.root()?;
        let res = self.run_output_with_retry(|| {
            let mut command = self.command();
            command.args(audit_cell_arguments());
            command.current_dir(&root);
            command
        })?;
        Ok(String::from_utf8(res.stdout)?)
    }

    pub fn audit_config(&mut self) -> anyhow::Result<String> {
        let root = self.root()?;
        let res = self.run_output_with_retry(|| {
            let mut command = self.command();
            command.args(audit_config_arguments());
            command.current_dir(&root);
            command
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

        let res = self.run_output_with_retry(|| {
            let mut command = self.command();
            command
                .args(targets_arguments())
                .arg("--output")
                .arg(output)
                .arg(at_file.clone())
                .args(extra_args);
            append_platform_config(&mut command);
            command
        })?;
        res.status.exit_result().context("buck2 targets failed")?;
        Ok(())
    }

    pub fn owners(
        &mut self,
        extra_args: &[String],
        changes: &[ProjectRelativePath],
    ) -> anyhow::Result<String> {
        assert!(!changes.is_empty());

        let (_file, at_file) = create_at_file_arg(changes, "\n")?;

        let root = self.root()?;
        let res = self.run_output_with_retry(|| {
            let mut command = self.command();
            command
                .arg("uquery")
                .arg("--json")
                .arg("owner(\"%s\")")
                .arg(&at_file)
                .args(extra_args);
            command.current_dir(&root);
            command
        })?;

        info!("Running owners query");
        Ok(String::from_utf8(res.stdout)?)
    }

    /// Reverse-deps of `seeds` within `universe_patterns`, via `buck2 uquery rdeps`.
    ///
    /// Buck2 defines `rdeps(universe, targets, ...)`: search for reverse dependencies
    /// of `targets` only within `universe`. Use `%Ss` so each `@` file is expanded
    /// into a single `set(...)` for one query (not `%s`, which runs one query per line
    /// and groups JSON output by input literal).
    pub fn uquery_rdeps(
        &mut self,
        seeds: &[TargetLabel],
        universe_patterns: &[String],
    ) -> anyhow::Result<Vec<TargetLabel>> {
        if seeds.is_empty() || universe_patterns.is_empty() {
            return Ok(Vec::new());
        }

        let seed_exprs: Vec<String> = seeds
            .iter()
            .map(|label| label.as_str().to_owned())
            .collect();
        let (_seed_file, at_seeds) = create_at_file_arg(&seed_exprs, "\n")?;
        let (_universe_file, at_universe) = create_at_file_arg(universe_patterns, "\n")?;

        let root = self.root()?;
        let res = self.run_output_with_retry(|| {
            let mut command = self.command();
            command
                .arg("uquery")
                .arg("--json")
                .arg("rdeps(%Ss, %Ss)")
                .arg(&at_universe)
                .arg(&at_seeds);
            append_platform_config(&mut command);
            command.current_dir(&root);
            command
        })?;

        parse_uquery_rdeps_labels(&String::from_utf8(res.stdout)?)
    }
}

fn parse_uquery_rdeps_labels(json_str: &str) -> anyhow::Result<Vec<TargetLabel>> {
    let raw: serde_json::Value = serde_json::from_str(json_str)?;
    let mut labels = match raw {
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(TargetLabel::new))
            .collect(),
        serde_json::Value::Object(map) => {
            // Multi-query `%s` groups results by input literal; collect targets from
            // values, not keys (keys may be universe patterns like `root//...`).
            if map.values().all(|v| v.is_array()) {
                map.into_values()
                    .flat_map(|v| {
                        v.as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|item| item.as_str().map(TargetLabel::new))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            } else {
                map.keys().map(|key| TargetLabel::new(key)).collect()
            }
        }
        _ => Vec::new(),
    };
    labels.sort();
    labels.dedup();
    Ok(labels)
}

fn should_retry_buck2_daemon(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("failed to connect to buck daemon")
        || lower.contains("no buckd.info timed out")
        || lower.contains("starting new buck2 daemon")
}

pub fn targets_arguments() -> &'static [&'static str] {
    &[
        "targets",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uquery_rdeps_labels_accepts_target_array() {
        let json = r#"["root//rk8s/lib:foo", "root//rk8s/bin:bar"]"#;
        let labels = parse_uquery_rdeps_labels(json).unwrap();
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&TargetLabel::new("root//rk8s/lib:foo")));
        assert!(labels.contains(&TargetLabel::new("root//rk8s/bin:bar")));
    }

    #[test]
    fn parse_uquery_rdeps_labels_flattens_multi_query_grouped_output() {
        let json = r#"{
            "root//rk8s/...": ["root//rk8s/lib:foo", "root//rk8s/bin:bar"],
            "root//other/...": ["root//other:lib"]
        }"#;
        let labels = parse_uquery_rdeps_labels(json).unwrap();
        assert_eq!(labels.len(), 3);
        assert!(!labels.iter().any(|l| l.as_str().contains("...")));
        assert!(labels.contains(&TargetLabel::new("root//rk8s/lib:foo")));
        assert!(labels.contains(&TargetLabel::new("root//other:lib")));
    }

    #[test]
    fn parse_uquery_rdeps_labels_reads_target_keys_from_attribute_map() {
        let json = r#"{
            "root//rk8s/lib:foo": {"name": "foo"},
            "root//rk8s/bin:bar": {"name": "bar"}
        }"#;
        let labels = parse_uquery_rdeps_labels(json).unwrap();
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&TargetLabel::new("root//rk8s/lib:foo")));
    }
}
