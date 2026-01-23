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
    collections::{HashMap, HashSet, hash_map::Entry},
    mem,
    sync::Arc,
};

use td_util_buck::{
    config::{is_buckconfig_change, should_exclude_bzl_file_from_transitive_impact_tracing},
    glob::GlobSpec,
    labels::Labels,
    target_map::TargetMap,
    targets::{BuckTarget, Targets},
    types::{CellPath, Glob, Package, RuleType, TargetLabel, TargetLabelKeyRef, TargetName},
};
use tracing::{info, warn};

use crate::repo::changes::Changes;

/// Given the state, which .bzl files have changed, either directly or by transitive dependencies
fn changed_bzl_files<'a>(
    state: &'a Targets,
    changes: &Changes,
    track_prelude_changes: bool,
) -> HashSet<&'a CellPath> {
    let mut rdeps: HashMap<&CellPath, Vec<&CellPath>> = HashMap::new();
    let mut todo = Vec::new();
    for x in state.imports() {
        // Always track regular rule changes, but ignore buck2 prelude changes
        // unless specifically requested.
        if !track_prelude_changes && x.file.is_prelude_bzl_file() {
            continue;
        }

        // There are certain macros whose impact we can track more accurately
        // without tracing transitively impacted bzl files e.g. via their changes
        // to package values, target attributes etc. This escape hatch
        // helps keep the blast radius of such included bzl files more reasonable.
        if should_exclude_bzl_file_from_transitive_impact_tracing(x.file.as_str()) {
            continue;
        }

        if changes.contains_cell_path(&x.file) {
            todo.push(&x.file);
        }
        for y in x.imports.iter() {
            rdeps.entry(y).or_default().push(&x.file);
        }
    }

    let mut res: HashSet<_> = todo.iter().copied().collect();
    while let Some(x) = todo.pop() {
        if let Some(rdep) = rdeps.get(x) {
            for r in rdep {
                if res.insert(*r) {
                    todo.push(*r);
                }
            }
        }
    }

    res
}

fn is_changed_ci_srcs(file_deps: &[Glob], changes: &Changes) -> bool {
    if file_deps.is_empty() || changes.is_empty() {
        return false;
    }
    let glob = GlobSpec::new(file_deps);
    changes.project_paths().any(|x| glob.matches(x))
}

/// If target has `ci_srcs_must_match` set, it could be picked only if changes match any
/// of the globs in `ci_srcs_must_match`.
/// Checks if the target is allowed to be considered based on this rule.
fn matches_ci_srcs_must_match(globs: &[Glob], changes: &Changes) -> bool {
    if globs.is_empty() || changes.is_empty() {
        return true;
    }
    let glob = GlobSpec::new(globs);
    changes.project_paths().any(|x| glob.matches(x))
}

/// The result of `immediate_changes`.
#[derive(Debug, Default)]
pub struct GraphImpact<'a> {
    /// Targets which changed, and whose change is expected to impact
    /// things that depend on them (they changed recursively).
    recursive: Vec<(&'a BuckTarget, ImpactTraceData)>,
    /// Targets which changed in a way that won't impact things recursively.
    /// Currently only package value changes and label changes.
    non_recursive: Vec<(&'a BuckTarget, ImpactTraceData)>,
    /// Targets which are removed.
    removed: Vec<(&'a BuckTarget, ImpactTraceData)>,
}

impl<'a> GraphImpact<'a> {
    pub fn from_recursive(recursive: Vec<(&'a BuckTarget, ImpactTraceData)>) -> Self {
        Self {
            recursive,
            ..Default::default()
        }
    }

    pub fn from_non_recursive(non_recursive: Vec<(&'a BuckTarget, ImpactTraceData)>) -> Self {
        Self {
            non_recursive,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.recursive.len() + self.non_recursive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&'a self) -> impl Iterator<Item = (&'a BuckTarget, ImpactTraceData)> {
        self.recursive
            .iter()
            .chain(self.non_recursive.iter())
            .cloned()
    }

    /// Sort all the fields, ensuring they are in a deterministic order.
    pub fn sort(&mut self) {
        self.recursive.sort_by_key(|(t, _)| t.label_key());
        self.non_recursive.sort_by_key(|(t, _)| t.label_key());
        self.removed.sort_by_key(|(t, _)| t.label_key());
    }
}

/// Contains metadata about why a target was impacted and information on the
/// targets place in the dependency graph.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct ImpactTraceData {
    /// The target name of the direct dependency which
    /// caused this target to be impacted.
    pub affected_dep: Arc<String>, // parent_target_name
    /// The target name of the dependency which actually changed
    pub root_cause_target: Arc<String>,
    /// The type of change that we detected in it.
    pub root_cause_reason: RootImpactKind,
    /// Whether the node is a root in the dependency graph.
    pub is_terminal: bool,
    /// New labels added to the target.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub added_labels: Vec<Arc<String>>,
    /// labels that were removed from the target.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub removed_labels: Vec<Arc<String>>,
}

impl ImpactTraceData {
    pub fn new(target: &BuckTarget, kind: RootImpactKind) -> Self {
        ImpactTraceData {
            affected_dep: Arc::new(String::new()),
            root_cause_target: Arc::new(format!(
                "{}:{}",
                target.package.as_str(),
                target.name.as_str()
            )),
            root_cause_reason: kind,
            is_terminal: false,
            added_labels: vec![],
            removed_labels: vec![],
        }
    }

    #[cfg(test)]
    pub fn testing() -> Self {
        ImpactTraceData {
            affected_dep: Arc::new("cell//foo:bar".to_owned()),
            root_cause_target: Arc::new("cell//baz:qux".to_owned()),
            root_cause_reason: RootImpactKind::Inputs,
            is_terminal: false,
            added_labels: vec![],
            removed_labels: vec![],
        }
    }
}

/// Categorization of the kind of immediate target change which caused BTD to
/// report a target as impacted. These reasons are propagated down through
/// rdeps, so they indicate that a target *or one of its dependencies* changed
/// in the indicated way.
#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
    parse_display::Display,
)]
#[serde(rename_all = "snake_case")]
#[display(style = "snake_case")]
/// When adding a new impact kind, ensure you also update
/// [`RootImpactKindReason`](https://fburl.com/code/qqk3abaz) in the `www` repository.
/// After making changes, run `meerkat` to apply the updates.
pub enum RootImpactKind {
    /// This target is new.
    New,
    /// This target was impacted because a target's package changed.
    Package,
    /// The hash of a target changed.
    Hash,
    /// Labels configured by users have changed
    Labels,
    /// The sources a target points at changed.
    Inputs,
    /// The `ci_srcs` of a target (used as additional triggers) changed.
    CiSrcs,
    /// The Buck rule used to define a target changed.
    Rule,
    /// The `buck.package_values` of a target changed.
    PackageValues,
    /// The target is removed
    Remove,
    /// When we want to manually rerun the target.
    ManualForRerun,
    /// Universal file is touched
    UniversalFile,
    /// We want to select all targets.
    SelectAll,
}

pub fn immediate_target_changes<'a>(
    base: &'a Targets,
    diff: &'a Targets,
    changes: &Changes,
    track_prelude_changes: bool,
) -> GraphImpact<'a> {
    if changes.cell_paths().any(is_buckconfig_change) {
        let mut ret = GraphImpact::from_non_recursive(
            diff.targets()
                .map(|t| (t, ImpactTraceData::new(t, RootImpactKind::UniversalFile)))
                .filter(|(t, _)| {
                    is_target_with_buck_dependencies(t)
                        || is_target_with_changed_ci_srcs(t, changes)
                })
                .filter(|(t, _)| matches_ci_srcs_must_match(&t.ci_srcs_must_match, changes))
                .collect(),
        );
        ret.sort();
        return ret;
    }

    tracing::debug!("Finding changes");

    // If there is no base graph, then everything is new.
    if base.len_targets_upperbound() == 0 {
        tracing::info!("All targets are new");
        let all_targets = diff
            .targets()
            .map(|t| (t, ImpactTraceData::new(t, RootImpactKind::SelectAll)))
            .collect();
        return GraphImpact::from_non_recursive(all_targets);
    }

    // Find those targets which are different
    let mut old = base.targets_by_label_key();

    // Find those .bzl files that have changed, including transitive changes
    let bzl_change = changed_bzl_files(diff, changes, track_prelude_changes);

    // Track the reason we determined a target to have changed
    let some_if = |reason, changed| if changed { Some(reason) } else { None };
    tracing::debug!("Iterating targets");
    let mut res = GraphImpact::default();
    for target in diff.targets() {
        let old_target = match old.remove(&target.label_key()) {
            Some(x) => x,
            None => {
                res.recursive
                    .push((target, ImpactTraceData::new(target, RootImpactKind::New)));
                continue;
            }
        };
        // "hidden feature" that allows using btd to find rdeps of a "package" (directory)
        // by including directory paths in the changes input
        let change_package = || {
            some_if(
                RootImpactKind::Package,
                changes.contains_package(&target.package),
            )
        };

        // Did the hash of the target change
        let change_hash = || some_if(RootImpactKind::Hash, old_target.hash != target.hash);

        // Did any of the target labels change
        let change_target_ci_labels = || {
            some_if(
                RootImpactKind::Labels,
                change_hash().is_some() && !ci_labels_unchanged(&target.labels, &old_target.labels),
            )
        };
        // Did the package labels change (this is separated from target labels to add package value changes to non-recursive)
        // Only package values we read are citadel.labels
        let change_package_ci_labels = || {
            some_if(
                RootImpactKind::Labels,
                !ci_labels_unchanged(
                    &target.package_values.labels,
                    &old_target.package_values.labels,
                ),
            )
        };

        // Did the package values change (this is only package labels changes that are not ci labels)
        let change_package_values = || {
            some_if(
                RootImpactKind::PackageValues,
                old_target.package_values != target.package_values,
            )
        };
        // Did any of the sources we point at change
        let change_inputs = || {
            some_if(
                RootImpactKind::Inputs,
                target.inputs.iter().any(|x| changes.contains_cell_path(x)),
            )
        };
        let change_ci_srcs = || {
            some_if(
                RootImpactKind::CiSrcs,
                is_changed_ci_srcs(&target.ci_srcs, changes)
                    && matches_ci_srcs_must_match(&target.ci_srcs_must_match, changes),
            )
        };
        // Did the rule we point at change
        let change_rule = || {
            some_if(
                RootImpactKind::Rule,
                !bzl_change.is_empty() && bzl_change.contains(&target.rule_type.file()),
            )
        };

        // The ordering here is important and goes from fine -> coarse.
        // We prioritize source file to cover code changes (most relevant)
        // Then target-level changes detected via label changes - these are added to non_recursive
        // so they won't impact things that depend on them recursively.
        // Then additional trigger conditions based on hash changes, CI sources, and package changes
        // which are added to recursive and will impact dependent targets.
        // Until rule-based matching, which is intentionally last because we can't infer true impact
        // just from the parsed graph and must schedule at least analysis.
        if let Some(reason) = change_inputs() {
            res.recursive
                .push((target, ImpactTraceData::new(target, reason)));
        } else if let Some(reason) = change_target_ci_labels() {
            res.non_recursive.push((
                target,
                ImpactTraceData {
                    // We want to add addded and removed labels to trace data so we can flag key differences later on, this only gets included if reason is labels.
                    added_labels: if reason == RootImpactKind::Labels {
                        let old_ci_labels = Labels::filter_ci_labels(&old_target.labels);
                        let new_ci_labels = Labels::filter_ci_labels(&target.labels);
                        let old_set: HashSet<_> =
                            old_ci_labels.iter().map(|l| l.as_str()).collect();
                        new_ci_labels
                            .iter()
                            .filter(|l| !old_set.contains(l.as_str()))
                            .map(|l| Arc::new(l.to_string()))
                            .collect()
                    } else {
                        vec![]
                    },
                    removed_labels: if reason == RootImpactKind::Labels {
                        let old_ci_labels = Labels::filter_ci_labels(&old_target.labels);
                        let new_ci_labels = Labels::filter_ci_labels(&target.labels);
                        let new_set: HashSet<_> =
                            new_ci_labels.iter().map(|l| l.as_str()).collect();
                        old_ci_labels
                            .iter()
                            .filter(|l| !new_set.contains(l.as_str()))
                            .map(|l| Arc::new(l.to_string()))
                            .collect()
                    } else {
                        vec![]
                    },
                    ..ImpactTraceData::new(target, reason)
                },
            ));
        } else if let Some(reason) = change_hash()
            .or_else(change_ci_srcs)
            .or_else(change_package)
            .or_else(change_rule)
        {
            res.recursive
                .push((target, ImpactTraceData::new(target, reason)));
        } else if let Some(reason) = change_package_ci_labels().or_else(change_package_values) {
            res.non_recursive.push((
                target,
                ImpactTraceData {
                    added_labels: if reason == RootImpactKind::Labels {
                        let old_ci_labels =
                            Labels::filter_ci_labels(&old_target.package_values.labels);
                        let new_ci_labels = Labels::filter_ci_labels(&target.package_values.labels);
                        let old_set: HashSet<_> =
                            old_ci_labels.iter().map(|l| l.as_str()).collect();
                        new_ci_labels
                            .iter()
                            .filter(|l| !old_set.contains(l.as_str()))
                            .map(|l| Arc::new(l.to_string()))
                            .collect()
                    } else {
                        vec![]
                    },
                    removed_labels: if reason == RootImpactKind::Labels {
                        let old_ci_labels =
                            Labels::filter_ci_labels(&old_target.package_values.labels);
                        let new_ci_labels = Labels::filter_ci_labels(&target.package_values.labels);
                        let new_set: HashSet<_> =
                            new_ci_labels.iter().map(|l| l.as_str()).collect();
                        old_ci_labels
                            .iter()
                            .filter(|l| !new_set.contains(l.as_str()))
                            .map(|l| Arc::new(l.to_string()))
                            .collect()
                    } else {
                        vec![]
                    },
                    ..ImpactTraceData::new(target, reason)
                },
            ));
        }
    }
    // We remove targets from `old` when iterating `diff` above.
    // At this point, only removed targets are left in `old`.
    res.removed = old
        .into_values()
        .map(|target| (target, ImpactTraceData::new(target, RootImpactKind::Remove)))
        .collect();

    // Sort to ensure deterministic output
    res.sort();
    res
}

pub fn is_ci_target(buck_target: &BuckTarget) -> bool {
    let ci_srcs_rule_types = ["ci_skycastle", "ci_sandcastle", "ci_translator_workflow"];

    ci_srcs_rule_types.contains(&buck_target.rule_type.short())
}

pub fn ci_labels_unchanged(labels: &Labels, old_labels: &Labels) -> bool {
    Labels::filter_ci_labels(labels).eq(&Labels::filter_ci_labels(old_labels))
}

pub fn is_target_with_changed_ci_srcs(buck_target: &BuckTarget, changes: &Changes) -> bool {
    if is_ci_target(buck_target) {
        return is_changed_ci_srcs(&buck_target.ci_srcs, changes);
    }
    true
}

pub fn is_target_with_buck_dependencies(buck_target: &BuckTarget) -> bool {
    if is_ci_target(buck_target) {
        !buck_target.ci_deps.is_empty()
    } else {
        true
    }
}

fn hint_applies_to(target: &BuckTarget) -> Option<(&Package, TargetName)> {
    // for hints, the name will be `foo//bar:ci_hint@baz` which means
    // we need to test `foo//bar:baz`.
    Some((
        &target.package,
        TargetName::new(target.name.as_str().strip_prefix("ci_hint@")?),
    ))
}
pub fn recursive_target_changes<'a>(
    diff: &'a Targets,
    changes: &Changes,
    impact: &GraphImpact<'a>,
    depth: Option<usize>,
    follow_rule_type: impl Fn(&RuleType) -> bool,
) -> Vec<Vec<(&'a BuckTarget, ImpactTraceData)>> {
    // Just an optimisation, but saves building the reverse mapping
    if impact.recursive.is_empty() && impact.removed.is_empty() {
        info!("No recursive target changes");
        let mut res = if impact.non_recursive.is_empty() {
            Vec::new()
        } else {
            vec![impact.non_recursive.clone()]
        };
        // We use a empty list sentinel to show nothing missing
        res.push(Vec::new());
        res.truncate(depth.unwrap_or(usize::MAX));
        return res;
    }

    // We expect most things will have at least one dependency, so a reasonable approximate size
    let mut rdeps: TargetMap<&BuckTarget> = TargetMap::with_capacity(diff.len_targets_upperbound());
    let mut hints: HashMap<(&Package, TargetName), TargetLabel> = HashMap::new();
    for target in diff
        .targets()
        .filter(|t| matches_ci_srcs_must_match(&t.ci_srcs_must_match, changes))
    {
        for d in target.deps.iter() {
            rdeps.insert(d, target)
        }
        for d in target.ci_deps.iter() {
            if let Some(label) = d.as_target_label() {
                if label.is_package_relative() {
                    rdeps.insert(&target.package.join(&label.target_name()), target);
                } else {
                    rdeps.insert(&label, target);
                }
            } else {
                rdeps.insert_pattern(d, target);
            }
        }
        if target.rule_type.short() == "ci_hint" {
            match hint_applies_to(target) {
                Some(dest) => {
                    hints.insert(dest, target.label());
                }
                None => warn!("`ci_hint` target has invalid name: `{}`", target.label()),
            }
        }
    }
    // We record the hints going through (while we don't have the targets to hand),
    // then fill them in later with this loop
    if !hints.is_empty() {
        for target in diff.targets() {
            if let Some(hint) = hints.remove(&(&target.package, target.name.clone())) {
                rdeps.insert(&hint, target);
                if hints.is_empty() {
                    break;
                }
            }
        }
    }

    // The code below is carefully optimised to avoid multiple lookups and reuse memory allocations.
    // We use `done` to record which elements have been queued for adding to the results, to avoid duplicates.
    // We use `todo` for things we are looping over that will become results at the end of this loop.
    // We use `next` for things we want to loop over in the next loop.
    // At the end of each loop, we add `todo` to the results and make `todo = next`.
    //
    // All non-recursive changes are already queued for adding to results, but haven't been recursively explored.
    // We record them with `done[target] = false` and add them to `next_silent` (which becomes `todo_silent`).
    // This ensures we iterate over them if reached recursively, but don't add them to results twice.

    let mut todo = impact.recursive.clone();
    let mut non_recursive_changes = impact.non_recursive.clone();

    let mut done: HashMap<TargetLabelKeyRef, bool> = impact
        .recursive
        .iter()
        .map(|(x, _)| (x.label_key(), true))
        .chain(
            impact
                .non_recursive
                .iter()
                .map(|(x, _)| (x.label_key(), false)),
        )
        .collect();

    let mut result = Vec::new();

    // Track targets depending on removed targets, but we don't add removed targets
    // to results
    let mut todo_silent: Vec<(&BuckTarget, ImpactTraceData)> = impact.removed.clone();
    let mut next_silent: Vec<(&BuckTarget, ImpactTraceData)> = Vec::new();

    fn add_result<'a>(
        results: &mut Vec<Vec<(&'a BuckTarget, ImpactTraceData)>>,
        mut items: Vec<(&'a BuckTarget, ImpactTraceData)>,
    ) {
        // Sort to ensure deterministic output
        items.sort_by_key(|(x, _)| x.label_key());
        results.push(items);
    }

    for _ in 0..depth.unwrap_or(usize::MAX) {
        if todo.is_empty() && todo_silent.is_empty() {
            if !non_recursive_changes.is_empty() {
                add_result(&mut result, non_recursive_changes);
            }
            break;
        }

        let mut next = Vec::new();

        for (lbl, reason) in todo.iter().chain(todo_silent.iter()) {
            if follow_rule_type(&lbl.rule_type) {
                let updated_reason = ImpactTraceData {
                    affected_dep: Arc::new(lbl.label().to_string()),
                    root_cause_target: reason.root_cause_target.clone(),
                    root_cause_reason: reason.root_cause_reason,
                    is_terminal: false,
                    added_labels: reason.added_labels.clone(),
                    removed_labels: reason.removed_labels.clone(),
                };
                for rdep in rdeps.get(&lbl.label()) {
                    match done.entry(rdep.label_key()) {
                        Entry::Vacant(e) => {
                            next.push((*rdep, updated_reason.clone()));
                            e.insert(true);
                        }
                        Entry::Occupied(mut e) => {
                            if !e.get() {
                                next_silent.push((*rdep, updated_reason.clone()));
                                e.insert(true);
                            }
                        }
                    }
                }
            }
        }
        if !non_recursive_changes.is_empty() {
            non_recursive_changes.extend(todo.iter().cloned());
            add_result(&mut result, mem::take(&mut non_recursive_changes));
        } else if !todo.is_empty() {
            add_result(&mut result, mem::take(&mut todo));
        }
        todo = next;

        // Do a swap so that we reuse the capacity of the buffer next time around
        mem::swap(&mut todo_silent, &mut next_silent);
        next_silent.clear();
    }

    // an empty todo list might be added to the result here, indicating to
    // the user (in Text output mode) that there are no additional levels
    add_result(&mut result, todo);
    annotate_terminal_nodes(&mut result, &rdeps);
    result
}

/// For all nodes that are affected, mark the ones which are terminal in the
/// target graph. We do not mark nodes cut off by depth as terminal.
fn annotate_terminal_nodes(
    result: &mut [Vec<(&BuckTarget, ImpactTraceData)>],
    rdeps: &TargetMap<&BuckTarget>,
) {
    for level in result.iter_mut() {
        for (target, trace) in level.iter_mut() {
            if rdeps.is_terminal_node(&target.label()) {
                trace.is_terminal = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use itertools::Itertools;
    use td_util::prelude::*;
    use td_util_buck::{
        cells::CellInfo,
        labels::Labels,
        targets::{BuckImport, TargetsEntry},
        types::{PackageValues, ProjectRelativePath, TargetHash, TargetPattern},
    };

    use super::*;
    use crate::repo::sapling::status::Status;

    fn basic_changes() -> Changes {
        Changes::testing(&[Status::Modified(CellPath::new("foo//irrelevant_file"))])
    }

    #[test]
    fn test_immediate_changes() {
        fn target(
            pkg: &str,
            name: &str,
            inputs: &[&CellPath],
            hash: &str,
            package_values: &PackageValues,
        ) -> TargetsEntry {
            TargetsEntry::Target(BuckTarget {
                inputs: inputs.iter().map(|x| (*x).clone()).collect(),
                hash: TargetHash::new(hash),
                package_values: package_values.clone(),
                ..BuckTarget::testing(name, pkg, "prelude//rules.bzl:cxx_library")
            })
        }

        let file1 = CellPath::new("foo//bar/file1.txt");
        let file2 = CellPath::new("foo//bar/file2.txt");
        let file3 = CellPath::new("foo//bar/file3.txt");
        let file4 = CellPath::new("foo//bar/file4.txt");

        // We could get a change because the hash changes or the input changes, or both
        // Or because the target is new.
        let default_package_value = PackageValues::new(&["default"]);
        let base = Targets::new(vec![
            target(
                "foo//bar",
                "aaa",
                &[&file1, &file2],
                "123",
                &default_package_value,
            ),
            target("foo//baz", "aaa", &[&file2], "123", &default_package_value),
            target("foo//bar", "bbb", &[&file3], "123", &default_package_value),
            target("foo//bar", "ccc", &[&file4], "123", &default_package_value),
            target("foo//bar", "ddd", &[], "123", &default_package_value),
            target("foo//bar", "eee", &[], "123", &default_package_value),
            target("foo//bar", "ggg", &[&file4], "123", &default_package_value),
            target(
                "foo//bar",
                "zzz",
                &[&file4],
                "123",
                &PackageValues::new(&["val1"]),
            ),
        ]);
        let diff = Targets::new(vec![
            target(
                "foo//bar",
                "aaa",
                &[&file1, &file4],
                "123",
                &default_package_value,
            ),
            target("foo//baz", "aaa", &[&file2], "123", &default_package_value),
            target("foo//bar", "bbb", &[&file3], "123", &default_package_value),
            target("foo//bar", "ccc", &[&file4], "123", &default_package_value),
            target("foo//bar", "ddd", &[], "123", &default_package_value),
            target("foo//bar", "fff", &[], "123", &default_package_value),
            target("foo//bar", "ggg", &[&file4], "321", &default_package_value),
            // only package value changed
            target(
                "foo//bar",
                "zzz",
                &[&file4],
                "123",
                &PackageValues::new(&["val2"]),
            ),
        ]);
        let res = immediate_target_changes(
            &base,
            &diff,
            &Changes::testing(&[
                Status::Modified(file1),
                Status::Added(file2),
                Status::Removed(file3),
            ]),
            false,
        );
        let recursive = res.recursive.map(|(x, _)| x.label().to_string());
        let non_recursive = res.non_recursive.map(|(x, _)| x.label().to_string());
        assert_eq!(
            recursive.map(|x| x.as_str()),
            &[
                "foo//bar:aaa",
                "foo//bar:bbb",
                "foo//bar:fff",
                "foo//bar:ggg",
                "foo//baz:aaa",
            ]
        );
        assert_eq!(non_recursive.map(|x| x.as_str()), &["foo//bar:zzz",]);
    }

    #[test]
    fn test_everything_is_immediate_on_universal_changes() {
        fn target(
            pkg: &str,
            name: &str,
            inputs: &[&CellPath],
            hash: &str,
            package_values: &PackageValues,
        ) -> TargetsEntry {
            TargetsEntry::Target(BuckTarget {
                inputs: inputs.iter().map(|x| (*x).clone()).collect(),
                hash: TargetHash::new(hash),
                package_values: package_values.clone(),
                ..BuckTarget::testing(name, pkg, "prelude//rules.bzl:cxx_library")
            })
        }

        let file1 = CellPath::new("fbsource//tools/buckconfigs/file1.bcfg");
        let file2 = CellPath::new("foo//bar/file2.txt");
        let file3 = CellPath::new("foo//bar/file3.txt");

        let default_package_value = PackageValues::new(&["default"]);
        let base = Targets::new(vec![
            target("foo//bar", "aaa", &[], "123", &default_package_value),
            target("foo//baz", "bbb", &[&file2], "123", &default_package_value),
            target("foo//bar", "ccc", &[&file3], "123", &default_package_value),
        ]);
        let res = immediate_target_changes(
            &base,
            &base,
            &Changes::testing(&[Status::Modified(file1), Status::Removed(file2)]),
            false,
        );
        assert!(res.recursive.is_empty());
        let non_recursive = res.non_recursive.map(|(x, _)| x.label().to_string());
        assert_eq!(
            non_recursive.map(|x| x.as_str()),
            &["foo//bar:aaa", "foo//bar:ccc", "foo//baz:bbb"]
        );
    }

    #[test]
    fn test_immediate_changes_with_removed() {
        fn target(
            pkg: &str,
            name: &str,
            inputs: &[&CellPath],
            hash: &str,
            package_values: &PackageValues,
        ) -> TargetsEntry {
            TargetsEntry::Target(BuckTarget {
                inputs: inputs.iter().map(|x| (*x).clone()).collect(),
                hash: TargetHash::new(hash),
                package_values: package_values.clone(),
                ..BuckTarget::testing(name, pkg, "prelude//rules.bzl:cxx_library")
            })
        }

        let file1 = CellPath::new("foo//bar/file1.txt");
        let file2 = CellPath::new("foo//bar/file2.txt");
        let file3 = CellPath::new("foo//bar/file3.txt");
        let file4 = CellPath::new("foo//bar/file4.txt");

        // We could get a change because the hash changes or the input changes, or both
        // Or because the target is new.
        let default_package_value = PackageValues::new(&["default"]);
        let base = Targets::new(vec![
            target(
                "foo//bar",
                "aaa",
                &[&file1, &file2],
                "123",
                &default_package_value,
            ),
            target("foo//baz", "aaa", &[&file2], "123", &default_package_value),
            target("foo//bar", "bbb", &[&file3], "123", &default_package_value),
            target("foo//bar", "ccc", &[&file4], "123", &default_package_value),
            target("foo//bar", "ddd", &[], "123", &default_package_value),
            target("foo//bar", "eee", &[], "123", &default_package_value),
            target("foo//bar", "ggg", &[&file4], "123", &default_package_value),
            target(
                "foo//bar",
                "zzz",
                &[&file4],
                "123",
                &PackageValues::new(&["val1"]),
            ),
        ]);
        let diff = Targets::new(vec![
            target(
                "foo//bar",
                "aaa",
                &[&file1, &file4],
                "123",
                &default_package_value,
            ),
            target("foo//baz", "aaa", &[&file2], "123", &default_package_value),
            target("foo//bar", "bbb", &[&file3], "123", &default_package_value),
            target("foo//bar", "ccc", &[&file4], "123", &default_package_value),
            target("foo//bar", "ddd", &[], "123", &default_package_value),
            target("foo//bar", "fff", &[], "123", &default_package_value),
            target("foo//bar", "ggg", &[&file4], "321", &default_package_value),
            // only package value changed
            target(
                "foo//bar",
                "zzz",
                &[&file4],
                "123",
                &PackageValues::new(&["val2"]),
            ),
        ]);
        let res = immediate_target_changes(
            &base,
            &diff,
            &Changes::testing(&[
                Status::Modified(file1),
                Status::Added(file2),
                Status::Removed(file3),
            ]),
            false,
        );
        let recursive = res.recursive.map(|(x, _)| x.label().to_string());
        let non_recursive = res.non_recursive.map(|(x, _)| x.label().to_string());
        let removed = res.removed.map(|(x, _)| x.label().to_string());
        assert_eq!(
            recursive.map(|x| x.as_str()),
            &[
                "foo//bar:aaa",
                "foo//bar:bbb",
                "foo//bar:fff",
                "foo//bar:ggg",
                "foo//baz:aaa",
            ]
        );
        assert_eq!(non_recursive.map(|x| x.as_str()), &["foo//bar:zzz",]);
        assert_eq!(removed.map(|x| x.as_str()), &["foo//bar:eee"]);
    }

    #[test]
    fn test_package_changes() {
        fn target(pkg: &str, name: &str, inputs: &[&CellPath], hash: &str) -> TargetsEntry {
            TargetsEntry::Target(BuckTarget {
                inputs: inputs.iter().map(|x| (*x).clone()).collect(),
                hash: TargetHash::new(hash),
                ..BuckTarget::testing(name, pkg, "prelude//rules.bzl:cxx_library")
            })
        }

        let file1 = CellPath::new("foo//bar/file1.txt");
        let file2 = CellPath::new("foo//bar/file2.txt");
        let package = CellPath::new("foo//bar");

        let base = Targets::new(vec![
            target("foo//bar", "aaa", &[&file1, &file2], "123"),
            target("foo//baz", "aaa", &[&file2], "123"),
            target("foo//bar", "bbb", &[], "123"),
        ]);
        let res = immediate_target_changes(
            &base,
            &base,
            &Changes::testing(&[Status::Modified(package)]),
            false,
        );
        let mut res = res.recursive.map(|(x, _)| x.label().to_string());
        res.sort();
        let res = res.map(|x| x.as_str());
        assert_eq!(&res, &["foo//bar:aaa", "foo//bar:bbb",]);
    }

    #[test]
    fn test_recursive_changes_non_recursive_only() {
        fn target(name: &str, deps: &[&str], package_values: &PackageValues) -> TargetsEntry {
            let pkg = Package::new("foo//");
            TargetsEntry::Target(BuckTarget {
                deps: deps.iter().map(|x| pkg.join(&TargetName::new(x))).collect(),
                package_values: package_values.clone(),
                ..BuckTarget::testing(name, pkg.as_str(), "prelude//rules.bzl:cxx_library")
            })
        }

        let diff = Targets::new(vec![target("a", &[], &PackageValues::new(&["val"]))]);

        let impact = GraphImpact {
            recursive: Vec::new(),
            non_recursive: vec![(diff.targets().next().unwrap(), ImpactTraceData::testing())],
            ..Default::default()
        };
        let res = recursive_target_changes(&diff, &basic_changes(), &impact, Some(2), |_| true);
        let res = res.map(|xs| {
            let mut xs = xs.map(|(x, _)| x.name.as_str());
            xs.sort();
            xs
        });
        assert_eq!(res, vec![vec!["a"], vec![]]);
    }

    #[test]
    fn test_recursive_changes_with_package_values_only_changes() {
        fn target(name: &str, deps: &[&str], package_values: &PackageValues) -> TargetsEntry {
            let pkg = Package::new("foo//");
            TargetsEntry::Target(BuckTarget {
                deps: deps.iter().map(|x| pkg.join(&TargetName::new(x))).collect(),
                package_values: package_values.clone(),
                ..BuckTarget::testing(name, pkg.as_str(), "prelude//rules.bzl:cxx_library")
            })
        }

        let diff = Targets::new(vec![
            target("a", &[], &PackageValues::new(&["val"])),
            target("b", &["a"], &PackageValues::new(&["non_recursive_change"])),
            target("c", &["b"], &PackageValues::new(&["val"])),
        ]);

        let impact = GraphImpact {
            recursive: vec![(diff.targets().next().unwrap(), ImpactTraceData::testing())],
            non_recursive: vec![(diff.targets().nth(1).unwrap(), ImpactTraceData::testing())],
            ..Default::default()
        };
        let res = recursive_target_changes(&diff, &basic_changes(), &impact, Some(2), |_| true);
        let res = res.map(|xs| {
            let mut xs = xs.map(|(x, _)| x.name.as_str());
            xs.sort();
            xs
        });
        assert_eq!(res, vec![vec!["a", "b"], vec!["c"]]);
    }

    #[test]
    fn test_recursive_changes() {
        // We should be able to deal with cycles, and pieces that aren't on the graph
        fn target(name: &str, deps: &[&str]) -> TargetsEntry {
            let pkg = Package::new("foo//");
            TargetsEntry::Target(BuckTarget {
                deps: deps.iter().map(|x| pkg.join(&TargetName::new(x))).collect(),
                ..BuckTarget::testing(name, pkg.as_str(), "prelude//rules.bzl:cxx_library")
            })
        }
        let diff = Targets::new(vec![
            target("a", &["1"]),
            target("1", &[]),
            target("b", &["a"]),
            target("c", &["a", "d"]),
            target("d", &["b", "c"]),
            target("e", &["d", "b"]),
            target("f", &["e"]),
            target("g", &["f", "1"]),
            target("z", &[]),
            target("package_value_only", &[]),
        ]);

        let impact = GraphImpact::from_recursive(vec![(
            diff.targets().next().unwrap(),
            ImpactTraceData::testing(),
        )]);
        let res = recursive_target_changes(&diff, &basic_changes(), &impact, Some(3), |_| true);
        let res = res.map(|xs| {
            let mut xs = xs.map(|(x, _)| x.name.as_str());
            xs.sort();
            xs
        });
        assert_eq!(
            res,
            vec![vec!["a"], vec!["b", "c"], vec!["d", "e"], vec!["f"],]
        );
    }

    #[test]
    fn test_recursive_with_removed_targets() {
        fn target(name: &str, deps: &[&str]) -> TargetsEntry {
            let pkg = Package::new("foo//");
            TargetsEntry::Target(BuckTarget {
                deps: deps.iter().map(|x| pkg.join(&TargetName::new(x))).collect(),
                ..BuckTarget::testing(name, pkg.as_str(), "prelude//rules.bzl:cxx_library")
            })
        }

        let removed = BuckTarget::testing("removed", "foo//", "prelude//rules.bzl:cxx_library");
        let diff = Targets::new(vec![
            target("a", &[]),
            target("b", &["a"]),
            target("c", &["a", "d"]),
            target("d", &["b"]),
            target("e", &[removed.name.as_str()]),
            target("f", &["e"]),
        ]);

        let changed_target = diff.targets().find(|t| t.name.as_str() == "a").unwrap();
        let impact = GraphImpact {
            recursive: vec![(
                changed_target,
                ImpactTraceData::new(changed_target, RootImpactKind::Inputs),
            )],
            removed: vec![(
                &removed,
                ImpactTraceData::new(&removed, RootImpactKind::Remove),
            )],
            ..Default::default()
        };
        let res = recursive_target_changes(&diff, &basic_changes(), &impact, Some(2), |_| true);
        let res = res.map(|xs| {
            let mut xs = xs.map(|(x, _)| x.name.as_str());
            xs.sort();
            xs
        });
        assert_eq!(res, vec![vec!["a"], vec!["b", "c", "e"], vec!["d", "f"]]);
    }

    #[test]
    fn test_recursive_relative_ci_deps() {
        let diff = Targets::new(vec![
            TargetsEntry::Target(BuckTarget {
                ci_deps: Box::new([TargetPattern::new(":dep")]),
                ..BuckTarget::testing("bar", "code//foo", "prelude//rules.bzl:cxx_library")
            }),
            TargetsEntry::Target(BuckTarget::testing(
                "dep",
                "code//foo",
                "prelude//rules.bzl:cxx_library",
            )),
            TargetsEntry::Target(BuckTarget::testing(
                "foo",
                "code//foo",
                "prelude//rules.bzl:cxx_library",
            )),
        ]);

        let change_target =
            BuckTarget::testing("dep", "code//foo", "prelude//rules.bzl:cxx_library");
        let impact =
            GraphImpact::from_recursive(vec![(&change_target, ImpactTraceData::testing())]);
        let res = recursive_target_changes(&diff, &basic_changes(), &impact, Some(1), |_| true);
        let res = res.map(|xs| {
            let mut xs = xs.map(|(x, _)| x.name.as_str());
            xs.sort();
            xs
        });
        assert_eq!(res, vec![vec!["dep"], vec!["bar"]]);
    }

    #[test]
    fn test_recursive_changes_custom_workflows() {
        let diff = Targets::new(vec![
            // ci_deps not affected, target ignored
            create_buck_target(
                "a",
                "ci_sandcastle",
                None,
                None,
                Some(&["foo//bar:other_dep"]),
            ),
            // ci_deps affected, target selected
            create_buck_target("b", "ci_skycastle", None, None, Some(&["foo//bar:dep"])),
            // ci_deps affected, ci_srcs_must_match matches, target included
            create_buck_target(
                "c",
                "ci_translator_workflow",
                None,
                Some(&["changed"]),
                Some(&["foo//bar:dep"]),
            ),
            // ci_deps affected, ci_srcs_must_match does not match, target ignored
            create_buck_target(
                "d",
                "ci_sandcastle",
                None,
                Some(&["missing"]),
                Some(&["foo//bar:dep"]),
            ),
        ]);

        let change_target =
            BuckTarget::testing("dep", "foo//bar", "prelude//rules.bzl:cxx_library");
        let impact =
            GraphImpact::from_recursive(vec![(&change_target, ImpactTraceData::testing())]);
        let changes = Changes::testing(&[Status::Modified(CellPath::new("foo//changed"))]);
        let res = recursive_target_changes(&diff, &changes, &impact, Some(1), |_| true);
        let res = res.map(|xs| {
            let mut xs = xs.map(|(x, _)| x.name.as_str());
            xs.sort();
            xs
        });
        assert_eq!(res, vec![vec!["dep"], vec!["b", "c"]]);
    }

    #[test]
    fn test_recursive_changes_returns_unique_targets() {
        fn target(name: &str, deps: &[&str]) -> TargetsEntry {
            let pkg = Package::new("foo//");
            TargetsEntry::Target(BuckTarget {
                deps: deps.iter().map(|x| pkg.join(&TargetName::new(x))).collect(),
                ..BuckTarget::testing(name, pkg.as_str(), "prelude//rules.bzl:cxx_library")
            })
        }
        let diff = Targets::new(vec![
            target("a", &["1"]),
            target("b", &["a", "c"]),
            target("1", &[]),
            target("c", &["a"]),
            target("d", &["a", "c"]),
        ]);

        let impact = GraphImpact::from_recursive(
            diff.targets()
                .take(2)
                .map(|x| (x, ImpactTraceData::testing()))
                .collect(),
        );
        let res = recursive_target_changes(&diff, &basic_changes(), &impact, None, |_| true);
        let res = res.map(|xs| xs.map(|(x, _)| x.name.as_str()));
        assert_eq!(res, vec![vec!["a", "b"], vec!["c", "d"], vec![]]);
    }

    #[test]
    fn test_prelude_rule_changes() {
        // prelude.bzl imports rules.bzl which imports foo.bzl
        let targets = Targets::new(vec![
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//prelude.bzl"),
                imports: Box::new([CellPath::new("prelude//native.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//native.bzl"),
                imports: Box::new([
                    CellPath::new("prelude//rules.bzl"),
                    CellPath::new("prelude//unrelated.bzl"),
                ]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//rules.bzl"),
                imports: Box::new([CellPath::new("prelude//utils.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//utils.bzl"),
                imports: Box::new([]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//unrelated.bzl"),
                imports: Box::new([]),
                package: None,
            }),
            TargetsEntry::Target(BuckTarget::testing(
                "foo",
                "code//bar",
                "prelude//rules.bzl:genrule",
            )),
        ]);
        let check = |file, check, expect: usize| {
            assert_eq!(
                immediate_target_changes(
                    &targets,
                    &targets,
                    &Changes::testing(&[Status::Modified(CellPath::new(file))]),
                    check,
                )
                .len(),
                expect
            )
        };
        check("prelude//rules.bzl", false, 0);
        check("prelude//rules.bzl", true, 1);
        check("prelude//utils.bzl", true, 1);
        check("prelude//prelude.bzl", true, 0);
        check("prelude//unrelated.bzl", true, 0);
    }

    #[test]
    fn test_prelude_change_impact_on_ci_udr() {
        let targets = Targets::new(vec![
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("fbcode//bar/BUCK"),
                imports: Box::new([
                    CellPath::new("fbsource//tools/target_determinator/macros/ci_sandcastle.bzl"),
                    CellPath::new("fbcode//my_rules.bzl"),
                ]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("fbcode//my_rules.bzl"),
                imports: Box::new([
                    CellPath::new("fbsource//tools/target_determinator/macros/ci_sandcastle.bzl"),
                    CellPath::new("prelude//prelude.bzl"),
                ]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("fbsource//tools/target_determinator/macros/ci_sandcastle.bzl"),
                imports: Box::new([CellPath::new("prelude//prelude.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//prelude.bzl"),
                imports: Box::new([CellPath::new("prelude//rules.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//rules.bzl"),
                imports: Box::new([CellPath::new("prelude//utils.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//utils.bzl"),
                imports: Box::new([]),
                package: None,
            }),
            TargetsEntry::Target(BuckTarget::testing(
                "ci",
                "fbcode//bar",
                "fbsource//tools/target_determinator/macros/ci_sandcastle.bzl:ci_sandcastle",
            )),
            TargetsEntry::Target(BuckTarget::testing(
                "foo",
                "fbcode//bar",
                "fbcode//my_rules.bzl:my_rule",
            )),
            TargetsEntry::Target(BuckTarget::testing(
                "baz",
                "fbcode//bar",
                "prelude//rules.bzl:genrule",
            )),
        ]);

        let check = |file, check, expect: usize| {
            let res = immediate_target_changes(
                &targets,
                &targets,
                &Changes::testing(&[Status::Modified(CellPath::new(file))]),
                check,
            );
            assert_eq!(res.len(), expect);
        };
        // Changes to non-prelude rules still tracks rule changes.
        check("fbcode//my_rules.bzl", false, 1);
        check("fbcode//my_rules.bzl", true, 1);
        // CI defs are excluded.
        check(
            "fbsource//tools/target_determinator/macros/ci_sandcastle.bzl",
            false,
            0,
        );
        check(
            "fbsource//tools/target_determinator/macros/ci_sandcastle.bzl",
            true,
            0,
        );
        // Changes from prelude are only tracked if the boolean is set.
        check("prelude//rules.bzl", false, 0);
        check("prelude//rules.bzl", true, 1);
        check("prelude//utils.bzl", false, 0);
        check("prelude//utils.bzl", true, 1);
    }

    #[test]
    fn test_non_prelude_rule_changes() {
        // test.bzl imports my_rules.bzl which imports prelude//rules.bzl
        let targets = Targets::new(vec![
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("fbcode//test.bzl"),
                imports: Box::new([CellPath::new("fbcode//my_rules.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("fbcode//my_rules.bzl"),
                imports: Box::new([CellPath::new("prelude//rules.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//rules.bzl"),
                imports: Box::new([CellPath::new("prelude//utils.bzl")]),
                package: None,
            }),
            TargetsEntry::Import(BuckImport {
                file: CellPath::new("prelude//utils.bzl"),
                imports: Box::new([]),
                package: None,
            }),
            TargetsEntry::Target(BuckTarget::testing(
                "foo",
                "fbcode//bar",
                "fbcode//my_rules.bzl:my_rule",
            )),
            TargetsEntry::Target(BuckTarget::testing(
                "baz",
                "fbcode//bar",
                "prelude//rules.bzl:genrule",
            )),
        ]);
        let check = |file, check, expect: usize| {
            assert_eq!(
                immediate_target_changes(
                    &targets,
                    &targets,
                    &Changes::testing(&[Status::Modified(CellPath::new(file))]),
                    check,
                )
                .len(),
                expect
            )
        };
        // Changes to non-prelude rules still tracks rule changes.
        check("fbcode//my_rules.bzl", false, 1);
        check("fbcode//my_rules.bzl", true, 1);
        // Changes from prelude are only tracked if the boolean is set.
        check("prelude//rules.bzl", false, 0);
        check("prelude//rules.bzl", true, 2);
        check("prelude//utils.bzl", false, 0);
        check("prelude//utils.bzl", true, 2);
    }

    #[test]
    fn test_file_deps() {
        // prelude.bzl imports rules.bzl which imports foo.bzl
        let targets = Targets::new(vec![TargetsEntry::Target(BuckTarget {
            ci_srcs: Box::new([Glob::new("test/*.txt")]),
            ..BuckTarget::testing("foo", "code//bar", "prelude//rules.bzl:genrule")
        })]);
        let check = |file, expect: usize| {
            assert_eq!(
                immediate_target_changes(
                    &targets,
                    &targets,
                    &Changes::testing(&[Status::Modified(CellPath::new(&format!("root//{file}")))]),
                    false,
                )
                .len(),
                expect
            )
        };
        check("prelude/rules.bzl", 0);
        check("test/foo.java", 0);
        check("test/foo.txt", 1);
    }

    #[test]
    fn test_package_values() {
        // prelude.bzl imports rules.bzl which imports foo.bzl
        let before = Targets::new(vec![TargetsEntry::Target(BuckTarget::testing(
            "foo",
            "code//bar",
            "prelude//rules.bzl:genrule",
        ))]);
        let after = Targets::new(vec![TargetsEntry::Target(BuckTarget {
            package_values: PackageValues::new(&["foo"]),
            ..BuckTarget::testing("foo", "code//bar", "prelude//rules.bzl:genrule")
        })]);
        // The hash of the target doesn't change, but the package.value does
        assert_eq!(
            immediate_target_changes(&before, &after, &Changes::testing(&[]), false).len(),
            1
        );
    }

    #[test]
    fn test_graph_with_node_cycles() {
        let src = CellPath::new("foo//src.txt");

        // You can get a graph which has cycles because the uquery graph has cycles, but cquery doesn't.
        // Or because the graph is broken but Buck2 won't see that with streaming targets.
        let targets = Targets::new(vec![
            TargetsEntry::Target(BuckTarget {
                deps: Box::new([TargetLabel::new("foo//:b")]),
                inputs: Box::new([src.clone()]),
                ..BuckTarget::testing("a", "foo//", "")
            }),
            TargetsEntry::Target(BuckTarget {
                deps: Box::new([TargetLabel::new("foo//:a")]),
                ..BuckTarget::testing("b", "foo//", "")
            }),
        ]);
        let changes = Changes::testing(&[Status::Modified(src)]);
        let mut impact = immediate_target_changes(&targets, &targets, &changes, false);
        assert_eq!(impact.recursive.len(), 1);

        assert_eq!(
            recursive_target_changes(&targets, &changes, &impact, None, |_| true)
                .iter()
                .flatten()
                .count(),
            2
        );
        impact.recursive.push((
            targets.targets().nth(1).unwrap(),
            ImpactTraceData::testing(),
        ));
        assert_eq!(
            recursive_target_changes(&targets, &changes, &impact, None, |_| true)
                .iter()
                .flatten()
                .count(),
            2
        );
    }

    #[test]
    fn test_recursive_changes_hint() {
        // We should be able to deal with cycles, and pieces that aren't on the graph
        let diff = Targets::new(vec![
            TargetsEntry::Target(BuckTarget {
                ..BuckTarget::testing(
                    "ci_hint@baz",
                    "foo//bar",
                    "fbsource//tools/target_determinator/macros/rules/ci_hint.bzl:ci_hint",
                )
            }),
            TargetsEntry::Target(BuckTarget {
                ..BuckTarget::testing("baz", "foo//bar", "prelude//rules.bzl:cxx_library")
            }),
        ]);

        let impact = GraphImpact::from_recursive(vec![(
            diff.targets().next().unwrap(),
            ImpactTraceData::testing(),
        )]);
        let res = recursive_target_changes(&diff, &basic_changes(), &impact, Some(3), |_| true);
        assert_eq!(res[0].len(), 1);
        assert_eq!(res[1].len(), 1);
        assert_eq!(res[1][0].0.name, TargetName::new("baz"));
        assert_eq!(res.iter().flatten().count(), 2);
    }

    #[test]
    fn test_terminal_nodes() {
        fn target(name: &str, deps: &[&str]) -> TargetsEntry {
            let pkg = Package::new("foo//");
            TargetsEntry::Target(BuckTarget {
                deps: deps.iter().map(|x| pkg.join(&TargetName::new(x))).collect(),
                ..BuckTarget::testing(name, pkg.as_str(), "prelude//rules.bzl:cxx_library")
            })
        }

        // a, x, y, z are terminal nodes.
        let entries = vec![
            target("a", &["b"]),
            target("b", &["c", "d"]),
            target("c", &["e", "f"]),
            target("d", &["e"]),
            target("e", &["g"]),
            target("f", &[]),
            target("g", &[]),
            target("x", &["c"]),
            target("y", &["d"]),
            target("z", &["g"]),
        ];
        let diff = Targets::new(entries);

        let check = |impact: &GraphImpact, depth: usize, expected: &[&str]| {
            let res =
                recursive_target_changes(&diff, &basic_changes(), impact, Some(depth), |_| true);
            let mut terminal = res
                .iter()
                .flatten()
                .filter(|(_, impact)| impact.is_terminal)
                .map(|(t, _)| t.name.as_str())
                .collect::<Vec<_>>();
            terminal.sort();
            assert_eq!(&terminal, expected);
        };

        let changes = GraphImpact::from_recursive(vec![(
            diff.targets().find(|t| t.name.as_str() == "g").unwrap(),
            ImpactTraceData::testing(),
        )]);
        // All terminal nodes are within traversal distance.
        check(&changes, 5, &["a", "x", "y", "z"]);
        // Due to truncated distance, only 1 terminal node is returned.
        check(&changes, 1, &["z"]);

        let changes = GraphImpact::from_recursive(vec![(
            diff.targets().find(|t| t.name.as_str() == "c").unwrap(),
            ImpactTraceData::testing(),
        )]);
        check(&changes, 5, &["a", "x"]);
        check(&changes, 1, &["x"]);
    }

    fn create_buck_target(
        name: &str,
        rule_type: &str,
        ci_srcs: Option<&[&str]>,
        ci_srcs_must_match: Option<&[&str]>,
        ci_deps: Option<&[&str]>,
    ) -> TargetsEntry {
        let bt = BuckTarget {
            name: TargetName::new(name),
            package: Package::new("myPackage"),
            package_values: PackageValues::default(),
            rule_type: RuleType::new(rule_type),
            oncall: None,
            deps: Box::new([]),
            inputs: Box::new([]),
            hash: TargetHash::new("myTargetHash"),
            labels: Labels::default(),
            ci_srcs: match ci_srcs {
                Some(srcs) => srcs.iter().map(|&src| Glob::new(src)).collect(),
                None => Box::new([]),
            },
            ci_srcs_must_match: match ci_srcs_must_match {
                Some(srcs) => srcs.iter().map(|&src| Glob::new(src)).collect(),
                None => Box::new([]),
            },
            ci_deps: match ci_deps {
                Some(deps) => deps.iter().map(|&dep| TargetPattern::new(dep)).collect(),
                None => Box::new([]),
            },
        };
        TargetsEntry::Target(bt)
    }

    #[test]
    fn test_immediate_target_changes_returns_correct_targets_when_buckconfig_changes() {
        fn create_test_targets() -> Targets {
            Targets::new(vec![
                create_buck_target("a", "cpp_binary", None, None, Some(&["dep1", "dep2"])),
                create_buck_target("b", "python_library", None, None, None),
                create_buck_target("c", "ci_translator_workflow", None, None, None), // Target contains no deps and no srcs and should be ignored
                create_buck_target("d", "ci_translator_workflow", None, None, Some(&["/dep"])),
                create_buck_target(
                    "e",
                    "ci_skycastle",
                    Some(&["path/to/changed/file2"]),
                    None,
                    None,
                ),
                create_buck_target("f", "ci_sandcastle", None, None, None),
                // Target contains ci_srcs_must_match which matches a change, target should be selected
                create_buck_target(
                    "g",
                    "ci_sandcastle",
                    Some(&["path/to/changed/file2"]),
                    Some(&["path/to/changed/file2"]),
                    None,
                ),
                // Target contains ci_srcs_must_match which matches a change, target should be selected
                create_buck_target(
                    "h",
                    "ci_skycastle",
                    None,
                    Some(&["path/to/changed/file2"]),
                    Some(&["/dep"]),
                ),
                // Target contains ci_srcs_must_match which does not match any changes, target should be ignored
                create_buck_target(
                    "i",
                    "ci_translator_workflow",
                    Some(&["path/to/changed/file2"]),
                    Some(&["path/to/changed/missing"]),
                    None,
                ),
                // Target contains ci_srcs_must_match which does not match any changes, target should be ignored
                create_buck_target(
                    "j",
                    "ci_sandcastle",
                    None,
                    Some(&["path/to/changed/missing"]),
                    Some(&["/dep"]),
                ),
            ])
        }
        fn create_buckconfig_changes() -> Changes {
            let cell_json = serde_json::json!(
                {
                    "fbsource//tools/buckconfigs": "/fbsource-common.bcfg",
                }
            );
            let cells = CellInfo::parse(&cell_json.to_string()).unwrap();
            Changes::new(
                &cells,
                vec![
                    Status::Modified(ProjectRelativePath::new("fbsource//tools/buckconfigs/")),
                    Status::Added(ProjectRelativePath::new("path/to/changed/file2")),
                ],
            )
            .unwrap()
        }

        // Initialize
        let test_targets = create_test_targets();
        let test_changes = create_buckconfig_changes();

        // Act
        let result = immediate_target_changes(&test_targets, &test_targets, &test_changes, true);

        // Verify
        let expected_target_names = vec!["a", "b", "d", "e", "g", "h"];
        let result_targets = result
            .iter()
            .map(|(target, _)| target.name.as_str())
            .collect_vec();
        assert_eq!(expected_target_names, result_targets)
    }

    #[test]
    fn test_immediate_target_changes_for_custom_workflows() {
        fn create_test_targets() -> Targets {
            Targets::new(vec![
                // No ci_srcs, target should be ignored
                create_buck_target("a", "ci_sandcastle", None, None, None),
                // ci_srcs matches a change, target should be selected
                create_buck_target(
                    "b",
                    "ci_skycastle",
                    Some(&["path/to/changed/file1"]),
                    None,
                    None,
                ),
                // ci_srcs matches a change, ci_srcs_must_match also matches, target should be selected
                create_buck_target(
                    "c",
                    "ci_translator_workflow",
                    Some(&["path/to/changed/file1"]),
                    Some(&["path/to/changed/file2"]),
                    None,
                ),
                // ci_srcs matches a change, ci_srcs_must_match does not match, target should be ignored
                create_buck_target(
                    "d",
                    "ci_sandcastle",
                    Some(&["path/to/changed/file1"]),
                    Some(&["path/to/changed/missing"]),
                    None,
                ),
            ])
        }

        // Initialize
        let test_targets = create_test_targets();
        let test_changes = Changes::new(
            &CellInfo::testing(),
            vec![
                Status::Added(ProjectRelativePath::new("path/to/changed/file1")),
                Status::Added(ProjectRelativePath::new("path/to/changed/file2")),
            ],
        )
        .unwrap();

        // Act
        let result = immediate_target_changes(&test_targets, &test_targets, &test_changes, true);

        // Verify
        let expected_target_names = vec!["b", "c"];
        let result_targets = result
            .iter()
            .map(|(target, _)| target.name.as_str())
            .collect_vec();
        assert_eq!(expected_target_names, result_targets)
    }

    fn run_is_target_with_dependency_test(
        rule_types: &[&str],
        deps: Option<&[&str]>,
        expected: bool,
    ) {
        fn create_buck_target(rule_type: &str, ci_deps: Option<&[&str]>) -> BuckTarget {
            BuckTarget {
                name: TargetName::new("myTargetName"),
                package: Package::new("myPackage"),
                package_values: PackageValues::default(),
                rule_type: RuleType::new(rule_type),
                oncall: None,
                deps: Box::new([]),
                inputs: Box::new([]),
                hash: TargetHash::new("myTargetHash"),
                labels: Labels::default(),
                ci_srcs: Box::new([]),
                ci_srcs_must_match: Box::new([]),
                ci_deps: match ci_deps {
                    Some(deps) => deps.iter().map(|&dep| TargetPattern::new(dep)).collect(),
                    None => Box::new([]),
                },
            }
        }

        let test_targets = rule_types
            .iter()
            .map(|&rule_type| create_buck_target(rule_type, deps))
            .collect::<Vec<_>>();

        for target in test_targets {
            assert_eq!(is_target_with_buck_dependencies(&target), expected);
        }
    }

    #[test]
    fn test_is_target_with_buck_dependencies_returns_true_when_deps_are_set() {
        let rule_types = ["ci_translator_workflow"];
        run_is_target_with_dependency_test(&rule_types, Some(&["ci_dep1", "ci_dep2"]), true);
    }

    #[test]
    fn test_is_target_with_buck_dependencies_returns_true_when_deps_are_not_set_and_is_custom_rule_type()
     {
        let rule_types = ["my_custom_rule_type"];
        run_is_target_with_dependency_test(&rule_types, None, true);
    }

    #[test]
    fn test_is_target_with_buck_dependencies_returns_false_when_deps_are_not_set() {
        let rule_types = ["ci_translator_workflow"];
        run_is_target_with_dependency_test(&rule_types, None, false);
    }

    fn run_is_target_with_ci_srcs_test(
        rule_types: &[&str],
        ci_srcs: Option<&[&str]>,
        changes: &Changes,
        expected: bool,
    ) {
        fn create_buck_target(rule_type: &str, ci_srcs: Option<&[&str]>) -> BuckTarget {
            BuckTarget {
                name: TargetName::new("myTargetName"),
                package: Package::new("myPackage"),
                package_values: PackageValues::default(),
                rule_type: RuleType::new(rule_type),
                oncall: None,
                deps: Box::new([]),
                inputs: Box::new([]),
                hash: TargetHash::new("myTargetHash"),
                labels: Labels::default(),
                ci_srcs: match ci_srcs {
                    Some(srcs) => srcs.iter().map(|&src| Glob::new(src)).collect(),
                    None => Box::new([]),
                },
                ci_srcs_must_match: Box::new([]),
                ci_deps: Box::new([]),
            }
        }

        let test_targets = rule_types
            .iter()
            .map(|&rule_type| create_buck_target(rule_type, ci_srcs))
            .collect::<Vec<_>>();

        for target in test_targets {
            assert_eq!(is_target_with_changed_ci_srcs(&target, changes), expected);
        }
    }

    #[test]
    fn test_is_target_with_ci_srcs_returns_true_when_ci_srcs_are_set() {
        let cell_info = CellInfo::testing();
        let project_paths = vec![
            Status::Modified(ProjectRelativePath::new("src1/lib.rs")),
            Status::Modified(ProjectRelativePath::new("src1/main.rs")),
        ];
        let changes = Changes::new(&cell_info, project_paths).unwrap();
        run_is_target_with_ci_srcs_test(
            &["ci_skycastle"],
            Some(&["src1/lib.rs", "src2"]),
            &changes,
            true,
        );
    }

    #[test]
    fn test_is_target_with_ci_srcs_returns_false_when_ci_srcs_are_not_set() {
        run_is_target_with_ci_srcs_test(&["ci_skycastle"], None, &Changes::default(), false);
    }

    #[test]
    fn test_is_target_with_ci_srcs_returns_true_for_non_ci_skycastle_rule_type() {
        run_is_target_with_ci_srcs_test(&["other_rule_type"], None, &Changes::default(), true);
    }
}
