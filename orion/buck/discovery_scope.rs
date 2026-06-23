//! Narrow `buck2 targets` discovery to subtrees touched by the change list (scheme A),
//! with optional `buck2 uquery rdeps` expansion over the full cell universe (scheme D).

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use api_model::buck2::{status::Status, types::ProjectRelativePath};

use crate::{
    cells::CellInfo,
    types::{CellName, CellPath},
};

/// Result of computing which target patterns to pass to `buck2 targets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryScope {
    /// Patterns such as `root//rk8s/...` for narrowed discovery.
    pub query_patterns: Vec<String>,
    /// `true` when patterns are a strict subset of the full cell scan.
    pub narrow: bool,
}

/// A monorepo subdirectory that carries its own `.buckconfig` (e.g. `rk8s/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubprojectBuckRoot {
    pub buck_root: PathBuf,
    pub strip_prefix: String,
}

/// When every changed path lives under one directory that has its own `.buckconfig`,
/// run Buck2 discovery from that directory so `PACKAGE` / cell config match the sub-project.
pub fn detect_subproject_buck_root(
    project_root: &Path,
    changes: &[Status<ProjectRelativePath>],
) -> Option<SubprojectBuckRoot> {
    if changes.is_empty() {
        return None;
    }

    let first = changes.first()?.get().as_str();
    let prefix = first
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())?;
    let prefix_with_slash = format!("{prefix}/");

    for change in changes {
        let path = change.get().as_str();
        if path != prefix && !path.starts_with(&prefix_with_slash) {
            return None;
        }
    }

    let buck_root = project_root.join(prefix);
    if !buck_root.join(".buckconfig").is_file() {
        return None;
    }

    Some(SubprojectBuckRoot {
        buck_root,
        strip_prefix: prefix.to_owned(),
    })
}

pub fn strip_subproject_path_prefix(
    path: &ProjectRelativePath,
    prefix: &str,
) -> ProjectRelativePath {
    let path_str = path.as_str();
    if path_str == prefix {
        return ProjectRelativePath::new("");
    }
    if let Some(rest) = path_str
        .strip_prefix(prefix)
        .and_then(|s| s.strip_prefix('/'))
    {
        ProjectRelativePath::new(rest)
    } else {
        path.clone()
    }
}

pub fn strip_subproject_changes(
    changes: &[Status<ProjectRelativePath>],
    prefix: &str,
) -> Vec<Status<ProjectRelativePath>> {
    changes
        .iter()
        .map(|change| {
            change
                .clone()
                .into_map(|path| strip_subproject_path_prefix(&path, prefix))
        })
        .collect()
}

/// Whether path-scoped discovery is enabled (scheme A + D).
///
/// Disable with `ORION_DISCOVERY_SCOPE=0`, `false`, `no`, or `off`.
pub fn discovery_scope_enabled() -> bool {
    match std::env::var("ORION_DISCOVERY_SCOPE") {
        Ok(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            !(normalized.is_empty()
                || normalized == "0"
                || normalized == "false"
                || normalized == "no"
                || normalized == "off")
        }
        Err(_) => true,
    }
}

/// Compute narrowed `buck2 targets` query patterns from the change list.
///
/// When narrowing applies, only first-level directory segments under each touched cell
/// are queried (e.g. `root//rk8s/...` instead of `root//...`). Repo-root `.buckconfig`
/// changes still trigger a full scan.
pub fn compute_discovery_scope(
    cells: &CellInfo,
    project_root: &Path,
    changes: &[Status<ProjectRelativePath>],
) -> DiscoveryScope {
    compute_discovery_scope_inner(cells, project_root, changes, discovery_scope_enabled())
}

fn compute_discovery_scope_inner(
    cells: &CellInfo,
    project_root: &Path,
    changes: &[Status<ProjectRelativePath>],
    enabled: bool,
) -> DiscoveryScope {
    let full_patterns = cells.get_all_cell_patterns(project_root);
    if !enabled || changes.is_empty() {
        return DiscoveryScope {
            query_patterns: full_patterns.clone(),
            narrow: false,
        };
    }

    let mut full_cell: HashSet<CellName> = HashSet::new();
    let mut segments: HashMap<CellName, HashSet<String>> = HashMap::new();
    let mut resolved_any = false;
    let mut force_full_scan = false;

    for change in changes {
        let path = change.get();
        let cell_path = match cells.unresolve(path) {
            Ok(cell_path) => cell_path,
            Err(_) => continue,
        };
        resolved_any = true;

        if is_repo_root_config(&cell_path) {
            force_full_scan = true;
            break;
        }

        if is_cell_wide_scan_path(&cell_path) {
            full_cell.insert(cell_path.cell());
            continue;
        }

        let path = cell_path.path();
        let rel = path.as_str();
        if let Some(segment) = first_path_segment(rel) {
            segments
                .entry(cell_path.cell())
                .or_default()
                .insert(segment.to_owned());
        } else {
            full_cell.insert(cell_path.cell());
        }
    }

    if !resolved_any || force_full_scan {
        return DiscoveryScope {
            query_patterns: full_patterns.clone(),
            narrow: false,
        };
    }

    let mut query_patterns = Vec::new();
    for cell_name in cells_with_changes(&full_cell, &segments) {
        let cell = cell_name.as_str();
        if full_cell.contains(&cell_name) {
            query_patterns.push(format!("{cell}//..."));
            continue;
        }
        if let Some(segs) = segments.get(&cell_name) {
            let mut sorted: Vec<_> = segs.iter().cloned().collect();
            sorted.sort();
            for segment in sorted {
                query_patterns.push(format!("{cell}//{segment}/..."));
            }
        }
    }

    if query_patterns.is_empty() {
        return DiscoveryScope {
            query_patterns: full_patterns.clone(),
            narrow: false,
        };
    }

    query_patterns.sort();
    query_patterns.dedup();

    let narrow = query_patterns != full_patterns;
    DiscoveryScope {
        query_patterns,
        narrow,
    }
}

fn cells_with_changes(
    full_cell: &HashSet<CellName>,
    segments: &HashMap<CellName, HashSet<String>>,
) -> Vec<CellName> {
    let mut cells: HashSet<_> = full_cell.iter().cloned().collect();
    cells.extend(segments.keys().cloned());
    let mut sorted: Vec<_> = cells.into_iter().collect();
    sorted.sort_by_key(|cell| cell.as_str().to_owned());
    sorted
}

fn is_repo_root_config(cell_path: &CellPath) -> bool {
    let path = cell_path.path();
    cell_path.cell().as_str() == "root"
        && matches!(path.as_str(), ".buckconfig" | ".buckroot" | ".buckversion")
}

/// Paths that affect an entire cell (top-level BUCK / cell config), but not repo-root config.
fn is_cell_wide_scan_path(cell_path: &CellPath) -> bool {
    let path = cell_path.path();
    let rel = path.as_str();
    if rel.is_empty() {
        return true;
    }
    if rel.contains('/') {
        return false;
    }
    matches!(
        rel,
        ".buckconfig" | ".buckroot" | ".buckversion" | "BUCK" | "TARGETS" | "BUCK.v2"
    )
}

fn first_path_segment(rel: &str) -> Option<&str> {
    rel.split('/').next().filter(|segment| !segment.is_empty())
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use super::*;

    fn test_cells(project_root: &Path) -> CellInfo {
        let cell_json = serde_json::json!({
            "root": project_root.to_str().unwrap(),
            "toolchains": project_root.join("toolchains").to_str().unwrap(),
            "prelude": project_root.join("prelude").to_str().unwrap(),
        });
        CellInfo::parse(&serde_json::to_string(&cell_json).unwrap()).unwrap()
    }

    fn temp_project_root() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("discovery_scope_test_{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(dir.join("toolchains")).unwrap();
        fs::create_dir_all(dir.join("prelude")).unwrap();
        fs::create_dir_all(dir.join("rk8s")).unwrap();
        dir
    }

    #[test]
    fn test_rk8s_only_changes_narrow_to_subtree() {
        let root = temp_project_root();
        let cells = test_cells(&root);
        let changes = vec![Status::Added(ProjectRelativePath::new(
            "rk8s/project/libfuse-fs/src/foo.rs",
        ))];

        let scope = compute_discovery_scope(&cells, &root, &changes);
        assert!(scope.narrow);
        assert_eq!(scope.query_patterns, vec!["root//rk8s/...".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_repo_root_buckconfig_forces_full_scan() {
        let root = temp_project_root();
        let cells = test_cells(&root);
        let changes = vec![Status::Modified(ProjectRelativePath::new(".buckconfig"))];

        let scope = compute_discovery_scope(&cells, &root, &changes);
        assert!(!scope.narrow);
        assert!(scope.query_patterns.contains(&"root//...".to_string()));
        assert!(scope
            .query_patterns
            .contains(&"toolchains//...".to_string()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_subproject_buckconfig_narrows_to_segment() {
        let root = temp_project_root();
        let cells = test_cells(&root);
        let changes = vec![Status::Modified(ProjectRelativePath::new(
            "rk8s/.buckconfig",
        ))];

        let scope = compute_discovery_scope(&cells, &root, &changes);
        assert!(scope.narrow);
        assert_eq!(scope.query_patterns, vec!["root//rk8s/...".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_toolchains_cell_change_includes_toolchains_pattern() {
        let root = temp_project_root();
        let cells = test_cells(&root);
        let changes = vec![Status::Modified(ProjectRelativePath::new(
            "toolchains/BUCK",
        ))];

        let scope = compute_discovery_scope(&cells, &root, &changes);
        assert!(scope.narrow);
        assert_eq!(scope.query_patterns, vec!["toolchains//...".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_multiple_segments_under_root() {
        let root = temp_project_root();
        let cells = test_cells(&root);
        let changes = vec![
            Status::Added(ProjectRelativePath::new("rk8s/a.rs")),
            Status::Added(ProjectRelativePath::new("third-party/b.rs")),
        ];

        let scope = compute_discovery_scope(&cells, &root, &changes);
        assert!(scope.narrow);
        assert_eq!(
            scope.query_patterns,
            vec![
                "root//rk8s/...".to_string(),
                "root//third-party/...".to_string(),
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_disabled_via_env_uses_full_scan() {
        let root = temp_project_root();
        let cells = test_cells(&root);
        let changes = vec![Status::Added(ProjectRelativePath::new(
            "rk8s/project/foo.rs",
        ))];

        let scope = compute_discovery_scope_inner(&cells, &root, &changes, false);
        assert!(!scope.narrow);
        assert!(scope.query_patterns.contains(&"root//...".to_string()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_empty_changes_use_full_scan() {
        let root = temp_project_root();
        let cells = test_cells(&root);

        let scope = compute_discovery_scope(&cells, &root, &[]);
        assert!(!scope.narrow);
        assert!(scope.query_patterns.contains(&"root//...".to_string()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_detect_subproject_buck_root_for_rk8s() {
        let root = temp_project_root();
        fs::write(root.join("rk8s/.buckconfig"), "[build]\n").unwrap();

        let changes = vec![Status::Added(ProjectRelativePath::new(
            "rk8s/project/common/src/lib.rs",
        ))];
        let sub = detect_subproject_buck_root(&root, &changes).expect("rk8s subproject");
        assert_eq!(sub.buck_root, root.join("rk8s"));
        assert_eq!(sub.strip_prefix, "rk8s");

        let stripped = strip_subproject_changes(&changes, "rk8s");
        assert_eq!(stripped[0].get().as_str(), "project/common/src/lib.rs");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_detect_subproject_rejects_mixed_prefixes() {
        let root = temp_project_root();
        fs::write(root.join("rk8s/.buckconfig"), "[build]\n").unwrap();

        let changes = vec![
            Status::Added(ProjectRelativePath::new("rk8s/a.rs")),
            Status::Added(ProjectRelativePath::new("third-party/b.rs")),
        ];
        assert!(detect_subproject_buck_root(&root, &changes).is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
