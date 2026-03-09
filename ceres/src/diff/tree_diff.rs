use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use common::{config::RenameConfig, errors::MegaError};
use git_internal::hash::ObjectHash;

use crate::{diff::similarity::content_similarity_percent, model::change_list::ClDiffFile};

// Keep a small per-destination candidate set before the global greedy pass.
const NUM_CANDIDATES_PER_DST: usize = 4;

// Lightweight candidate entry used while pairing deleted and new paths.
#[derive(Clone)]
struct PathHash {
    path: PathBuf,
    hash: ObjectHash,
}

// Final relocation pair chosen from the deleted/new candidate pools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityPair {
    deleted_idx: usize,
    new_idx: usize,
    similarity: u8,
}

// Temporary relocation candidate with an extra basename preference signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityCandidate {
    deleted_idx: usize,
    new_idx: usize,
    similarity: u8,
    name_score: u8,
}

// Runtime knobs that shape the Git-like rename matching flow.
#[derive(Clone, Copy)]
struct RenameHeuristics {
    similarity_threshold: u8,
    rename_limit: usize,
}

impl RenameHeuristics {
    // Normalize external config into the internal matcher settings.
    fn from_config(config: &RenameConfig) -> Self {
        Self {
            similarity_threshold: config.similarity_threshold.min(100),
            rename_limit: config.rename_limit,
        }
    }
}

pub fn calculate_tree_diff_basic(
    old_files: Vec<(PathBuf, ObjectHash)>,
    new_files: Vec<(PathBuf, ObjectHash)>,
) -> Result<Vec<ClDiffFile>, MegaError> {
    calculate_tree_diff_internal(old_files, new_files, None)
}

pub fn calculate_tree_diff_with_blobs(
    old_files: Vec<(PathBuf, ObjectHash)>,
    new_files: Vec<(PathBuf, ObjectHash)>,
    rename_config: &RenameConfig,
    blob_cache: &HashMap<ObjectHash, Vec<u8>>,
) -> Result<Vec<ClDiffFile>, MegaError> {
    let heuristics = RenameHeuristics::from_config(rename_config);
    calculate_tree_diff_internal(old_files, new_files, Some((heuristics, blob_cache)))
}

fn calculate_tree_diff_internal(
    old_files: Vec<(PathBuf, ObjectHash)>,
    new_files: Vec<(PathBuf, ObjectHash)>,
    similarity_input: Option<(RenameHeuristics, &HashMap<ObjectHash, Vec<u8>>)>,
) -> Result<Vec<ClDiffFile>, MegaError> {
    // Phase 1: build the path-level diff before any relocation matching.
    let old_files: HashMap<PathBuf, ObjectHash> = old_files.into_iter().collect();
    let new_files: HashMap<PathBuf, ObjectHash> = new_files.into_iter().collect();
    let unions: HashSet<PathBuf> = old_files.keys().chain(new_files.keys()).cloned().collect();
    let mut base_res = vec![];
    for path in unions {
        let old_hash = old_files.get(&path);
        let new_hash = new_files.get(&path);
        match (old_hash, new_hash) {
            (None, None) => {}
            (None, Some(new)) => base_res.push(ClDiffFile::New(path, *new)),
            (Some(old), None) => base_res.push(ClDiffFile::Deleted(path, *old)),
            (Some(old), Some(new)) => {
                if old == new {
                    continue;
                }
                base_res.push(ClDiffFile::Modified(path, *old, *new));
            }
        }
    }

    // Keep only deleted/new entries as relocation candidates for later phases.
    let mut deleted_entries = Vec::new();
    let mut new_entries = Vec::new();
    for item in &base_res {
        match item {
            ClDiffFile::Deleted(path, hash) => {
                deleted_entries.push(PathHash {
                    path: path.clone(),
                    hash: *hash,
                });
            }
            ClDiffFile::New(path, hash) => {
                new_entries.push(PathHash {
                    path: path.clone(),
                    hash: *hash,
                });
            }
            _ => {}
        }
    }
    deleted_entries.sort_by(|left, right| left.path.cmp(&right.path));
    new_entries.sort_by(|left, right| left.path.cmp(&right.path));

    // Phase 2: lock in exact hash matches before content-based pairing.
    let mut consumed_deleted_idx: HashSet<usize> = HashSet::new();
    let mut consumed_new_idx: HashSet<usize> = HashSet::new();
    let mut relocated_items = Vec::new();
    let exact_pairs = collect_exact_match_pairs(&deleted_entries, &new_entries);
    consume_pairs(
        &exact_pairs,
        &deleted_entries,
        &new_entries,
        &mut consumed_deleted_idx,
        &mut consumed_new_idx,
        &mut relocated_items,
    );

    if let Some((heuristics, blob_cache)) = similarity_input {
        // Phase 3: score the remaining deleted/new candidates with heuristics and content.
        let unmatched_deleted: Vec<usize> = (0..deleted_entries.len())
            .filter(|idx| !consumed_deleted_idx.contains(idx))
            .collect();
        let unmatched_new: Vec<usize> = (0..new_entries.len())
            .filter(|idx| !consumed_new_idx.contains(idx))
            .collect();

        let selected_pairs = collect_phase_three_pairs(
            &deleted_entries,
            &new_entries,
            &unmatched_deleted,
            &unmatched_new,
            blob_cache,
            heuristics,
        );
        consume_pairs(
            &selected_pairs,
            &deleted_entries,
            &new_entries,
            &mut consumed_deleted_idx,
            &mut consumed_new_idx,
            &mut relocated_items,
        );
    }

    // Finalize: replace the consumed delete/new pairs with relocation results.
    let (relocated_old_paths, relocated_new_paths) = collect_relocated_paths(&relocated_items);
    let mut res = Vec::new();
    for item in base_res {
        match item {
            ClDiffFile::Deleted(path, _) if relocated_old_paths.contains(&path) => {}
            ClDiffFile::New(path, _) if relocated_new_paths.contains(&path) => {}
            _ => res.push(item),
        }
    }
    res.extend(relocated_items);

    res.sort_by(|a, b| {
        a.path()
            .cmp(b.path())
            .then_with(|| a.kind_weight().cmp(&b.kind_weight()))
    });
    Ok(res)
}

// Mark selected candidate pairs as consumed and convert them into relocation diffs.
fn consume_pairs(
    pairs: &[SimilarityPair],
    deleted_entries: &[PathHash],
    new_entries: &[PathHash],
    consumed_deleted_idx: &mut HashSet<usize>,
    consumed_new_idx: &mut HashSet<usize>,
    relocated_items: &mut Vec<ClDiffFile>,
) {
    for pair in pairs {
        if consumed_deleted_idx.contains(&pair.deleted_idx)
            || consumed_new_idx.contains(&pair.new_idx)
        {
            continue;
        }

        let old_entry = &deleted_entries[pair.deleted_idx];
        let new_entry = &new_entries[pair.new_idx];
        if old_entry.path == new_entry.path {
            continue;
        }

        consumed_deleted_idx.insert(pair.deleted_idx);
        consumed_new_idx.insert(pair.new_idx);
        relocated_items.push(build_relocated_diff(
            old_entry.path.clone(),
            new_entry.path.clone(),
            old_entry.hash,
            new_entry.hash,
            pair.similarity,
        ));
    }
}

// Rebuild the old/new path sets that should disappear once relocation items are emitted.
fn collect_relocated_paths(relocated_items: &[ClDiffFile]) -> (HashSet<PathBuf>, HashSet<PathBuf>) {
    let mut old_paths = HashSet::new();
    let mut new_paths = HashSet::new();

    for item in relocated_items {
        match item {
            ClDiffFile::Renamed(old_path, new_path, _, _, _)
            | ClDiffFile::Moved(old_path, new_path, _, _, _) => {
                old_paths.insert(old_path.clone());
                new_paths.insert(new_path.clone());
            }
            _ => {}
        }
    }

    (old_paths, new_paths)
}

// Classify a relocation as rename or move based on whether the parent directory changed.
fn build_relocated_diff(
    old_path: PathBuf,
    new_path: PathBuf,
    old_hash: ObjectHash,
    new_hash: ObjectHash,
    similarity: u8,
) -> ClDiffFile {
    if old_path.parent() == new_path.parent() {
        ClDiffFile::Renamed(old_path, new_path, old_hash, new_hash, similarity)
    } else {
        ClDiffFile::Moved(old_path, new_path, old_hash, new_hash, similarity)
    }
}

// Pair deleted/new entries that already share the exact same blob hash.
fn collect_exact_match_pairs(
    deleted_entries: &[PathHash],
    new_entries: &[PathHash],
) -> Vec<SimilarityPair> {
    let mut deleted_by_hash: HashMap<ObjectHash, Vec<usize>> = HashMap::new();
    for (idx, item) in deleted_entries.iter().enumerate() {
        deleted_by_hash.entry(item.hash).or_default().push(idx);
    }

    let mut used_deleted = HashSet::new();
    let mut pairs = Vec::new();
    for (new_idx, new_entry) in new_entries.iter().enumerate() {
        let Some(candidate_deleted) = deleted_by_hash.get(&new_entry.hash) else {
            continue;
        };

        let mut best_deleted: Option<usize> = None;
        for &deleted_idx in candidate_deleted {
            if used_deleted.contains(&deleted_idx) {
                continue;
            }
            let old_entry = &deleted_entries[deleted_idx];
            if old_entry.path == new_entry.path {
                continue;
            }

            let should_replace = match best_deleted {
                None => true,
                Some(best_idx) => {
                    let candidate_name = basename_same(&old_entry.path, &new_entry.path);
                    let best_name = basename_same(&deleted_entries[best_idx].path, &new_entry.path);
                    (candidate_name && !best_name)
                        || (candidate_name == best_name
                            && old_entry.path < deleted_entries[best_idx].path)
                }
            };
            if should_replace {
                best_deleted = Some(deleted_idx);
            }
        }

        if let Some(deleted_idx) = best_deleted {
            used_deleted.insert(deleted_idx);
            pairs.push(SimilarityPair {
                deleted_idx,
                new_idx,
                similarity: 100,
            });
        }
    }
    pairs
}

// Resolve inexact relocations with a basename-first pass and a bounded similarity scan.
fn collect_phase_three_pairs(
    deleted_entries: &[PathHash],
    new_entries: &[PathHash],
    unmatched_deleted: &[usize],
    unmatched_new: &[usize],
    blob_cache: &HashMap<ObjectHash, Vec<u8>>,
    heuristics: RenameHeuristics,
) -> Vec<SimilarityPair> {
    // Give same-basename moves the first chance to match.
    let mut selected_pairs = collect_basename_pairs(
        deleted_entries,
        new_entries,
        unmatched_deleted,
        unmatched_new,
        blob_cache,
        heuristics,
    );

    let used_deleted: HashSet<usize> = selected_pairs.iter().map(|pair| pair.deleted_idx).collect();
    let used_new: HashSet<usize> = selected_pairs.iter().map(|pair| pair.new_idx).collect();
    let remaining_deleted: Vec<usize> = unmatched_deleted
        .iter()
        .copied()
        .filter(|idx| !used_deleted.contains(idx))
        .collect();
    let remaining_new: Vec<usize> = unmatched_new
        .iter()
        .copied()
        .filter(|idx| !used_new.contains(idx))
        .collect();

    // Bound the expensive inexact matrix on large candidate sets.
    if should_skip_inexact_phase(
        remaining_new.len(),
        remaining_deleted.len(),
        heuristics.rename_limit,
    ) {
        return selected_pairs;
    }

    let mut candidates = Vec::new();
    for &new_idx in &remaining_new {
        // Keep only the strongest sources for each destination before the global pass.
        let mut per_dst_candidates = Vec::new();
        let new_entry = &new_entries[new_idx];
        for &deleted_idx in &remaining_deleted {
            let old_entry = &deleted_entries[deleted_idx];
            let Some(similarity) = compute_similarity(old_entry, new_entry, blob_cache, heuristics)
            else {
                continue;
            };
            if similarity < heuristics.similarity_threshold {
                continue;
            }

            per_dst_candidates.push(SimilarityCandidate {
                deleted_idx,
                new_idx,
                similarity,
                name_score: basename_same(&old_entry.path, &new_entry.path) as u8,
            });
        }

        per_dst_candidates
            .sort_by(|left, right| compare_candidates(left, right, deleted_entries, new_entries));
        per_dst_candidates.truncate(NUM_CANDIDATES_PER_DST);
        candidates.extend(per_dst_candidates);
    }

    candidates.sort_by(|left, right| compare_candidates(left, right, deleted_entries, new_entries));
    let mut used_deleted = used_deleted;
    let mut used_new = used_new;
    for candidate in candidates {
        if used_deleted.contains(&candidate.deleted_idx) || used_new.contains(&candidate.new_idx) {
            continue;
        }
        used_deleted.insert(candidate.deleted_idx);
        used_new.insert(candidate.new_idx);
        selected_pairs.push(SimilarityPair {
            deleted_idx: candidate.deleted_idx,
            new_idx: candidate.new_idx,
            similarity: candidate.similarity,
        });
    }

    selected_pairs
}

// Prefer unique basename matches before the broader similarity matrix runs.
fn collect_basename_pairs(
    deleted_entries: &[PathHash],
    new_entries: &[PathHash],
    unmatched_deleted: &[usize],
    unmatched_new: &[usize],
    blob_cache: &HashMap<ObjectHash, Vec<u8>>,
    heuristics: RenameHeuristics,
) -> Vec<SimilarityPair> {
    let deleted_by_basename = build_unique_basename_index(deleted_entries, unmatched_deleted);
    let new_by_basename = build_unique_basename_index(new_entries, unmatched_new);

    let mut basename_keys: Vec<String> = deleted_by_basename
        .keys()
        .filter(|basename| new_by_basename.contains_key(*basename))
        .cloned()
        .collect();
    basename_keys.sort();

    let mut candidates = Vec::new();
    for basename in basename_keys {
        let deleted_idx = deleted_by_basename[&basename];
        let new_idx = new_by_basename[&basename];

        let old_entry = &deleted_entries[deleted_idx];
        let new_entry = &new_entries[new_idx];
        let Some(similarity) = compute_similarity(old_entry, new_entry, blob_cache, heuristics)
        else {
            continue;
        };
        if similarity < heuristics.similarity_threshold {
            continue;
        }

        candidates.push(SimilarityCandidate {
            deleted_idx,
            new_idx,
            similarity,
            name_score: 1,
        });
    }

    candidates.sort_by(|left, right| compare_candidates(left, right, deleted_entries, new_entries));
    let mut used_deleted = HashSet::new();
    let mut used_new = HashSet::new();
    let mut pairs = Vec::new();
    for candidate in candidates {
        if used_deleted.contains(&candidate.deleted_idx) || used_new.contains(&candidate.new_idx) {
            continue;
        }
        used_deleted.insert(candidate.deleted_idx);
        used_new.insert(candidate.new_idx);
        pairs.push(SimilarityPair {
            deleted_idx: candidate.deleted_idx,
            new_idx: candidate.new_idx,
            similarity: candidate.similarity,
        });
    }
    pairs
}

// Index only basenames that appear exactly once in the provided candidate slice.
fn build_unique_basename_index(entries: &[PathHash], indices: &[usize]) -> HashMap<String, usize> {
    let mut seen = HashMap::new();
    let mut duplicates = HashSet::new();

    for &idx in indices {
        let Some(basename) = basename_key(&entries[idx].path) else {
            continue;
        };
        if seen.insert(basename.clone(), idx).is_some() {
            duplicates.insert(basename);
        }
    }

    for duplicate in duplicates {
        seen.remove(&duplicate);
    }
    seen
}

// Extract the file name used by basename-based heuristics.
fn basename_key(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

// Check whether two paths share the same file name.
fn basename_same(old_path: &Path, new_path: &Path) -> bool {
    basename_key(old_path) == basename_key(new_path)
}

// Compute a Git-like similarity score when both blobs are available.
fn compute_similarity(
    old_entry: &PathHash,
    new_entry: &PathHash,
    blob_cache: &HashMap<ObjectHash, Vec<u8>>,
    heuristics: RenameHeuristics,
) -> Option<u8> {
    if old_entry.hash == new_entry.hash {
        return Some(100);
    }

    let old_bytes = blob_cache.get(&old_entry.hash)?;
    let new_bytes = blob_cache.get(&new_entry.hash)?;

    Some(content_similarity_percent(
        old_bytes,
        new_bytes,
        heuristics.similarity_threshold,
    ))
}

// Skip the exhaustive matrix once the candidate set grows past the rename limit.
fn should_skip_inexact_phase(
    num_destinations: usize,
    num_sources: usize,
    rename_limit: usize,
) -> bool {
    if rename_limit == 0 {
        return false;
    }

    (num_destinations as u128 * num_sources as u128) > (rename_limit as u128 * rename_limit as u128)
}

// Sort by similarity, basename preference, old path, then new path.
fn compare_candidates(
    left: &SimilarityCandidate,
    right: &SimilarityCandidate,
    deleted_entries: &[PathHash],
    new_entries: &[PathHash],
) -> std::cmp::Ordering {
    right
        .similarity
        .cmp(&left.similarity)
        .then_with(|| right.name_score.cmp(&left.name_score))
        .then_with(|| {
            deleted_entries[left.deleted_idx]
                .path
                .cmp(&deleted_entries[right.deleted_idx].path)
        })
        .then_with(|| {
            new_entries[left.new_idx]
                .path
                .cmp(&new_entries[right.new_idx].path)
        })
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, str::FromStr};

    use common::config::RenameConfig;

    use super::{calculate_tree_diff_basic, calculate_tree_diff_with_blobs};
    use crate::model::change_list::ClDiffFile;

    fn hash(value: &str) -> git_internal::hash::ObjectHash {
        git_internal::hash::ObjectHash::from_str(value).unwrap()
    }

    #[test]
    fn exact_matching_prefers_same_basename_when_hashes_repeat() {
        let repeated_hash = hash("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let diff = calculate_tree_diff_basic(
            vec![
                (PathBuf::from("src/foo.rs"), repeated_hash),
                (PathBuf::from("src/bar.rs"), repeated_hash),
            ],
            vec![
                (PathBuf::from("pkg/bar.rs"), repeated_hash),
                (PathBuf::from("pkg/foo.rs"), repeated_hash),
            ],
        )
        .unwrap();

        assert_eq!(diff.len(), 2);
        assert!(matches!(
            &diff[0],
            ClDiffFile::Moved(old_path, new_path, _, _, 100)
                if old_path == &PathBuf::from("src/bar.rs")
                    && new_path == &PathBuf::from("pkg/bar.rs")
        ));
        assert!(matches!(
            &diff[1],
            ClDiffFile::Moved(old_path, new_path, _, _, 100)
                if old_path == &PathBuf::from("src/foo.rs")
                    && new_path == &PathBuf::from("pkg/foo.rs")
        ));
    }

    #[test]
    fn basename_matching_runs_before_rename_limit_cutoff() {
        let old_main = hash("1111111111111111111111111111111111111111");
        let old_other = hash("2222222222222222222222222222222222222222");
        let new_main = hash("3333333333333333333333333333333333333333");
        let mut blob_cache = HashMap::new();
        blob_cache.insert(
            old_main,
            b"fn main() {\n    let stable = 7;\n    println!(\"stable\");\n    println!(\"old\");\n}\n".to_vec(),
        );
        blob_cache.insert(old_other, b"unrelated old file\n".to_vec());
        blob_cache.insert(
            new_main,
            b"fn main() {\n    let stable = 7;\n    println!(\"stable\");\n    println!(\"new\");\n}\n".to_vec(),
        );

        let diff = calculate_tree_diff_with_blobs(
            vec![
                (PathBuf::from("src/main.rs"), old_main),
                (PathBuf::from("src/other.txt"), old_other),
            ],
            vec![(PathBuf::from("app/main.rs"), new_main)],
            &RenameConfig {
                similarity_threshold: 50,
                rename_limit: 1,
                ..RenameConfig::default()
            },
            &blob_cache,
        )
        .unwrap();

        assert_eq!(diff.len(), 2);
        assert!(diff.iter().any(|item| {
            matches!(
                item,
                ClDiffFile::Moved(old_path, new_path, _, _, similarity)
                    if old_path == &PathBuf::from("src/main.rs")
                        && new_path == &PathBuf::from("app/main.rs")
                        && *similarity >= 50
            )
        }));
        assert!(diff.iter().any(|item| {
            matches!(item, ClDiffFile::Deleted(path, _) if path == &PathBuf::from("src/other.txt"))
        }));
    }

    #[test]
    fn rename_limit_skips_exhaustive_inexact_matching() {
        let old_a = hash("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let old_b = hash("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let new_c = hash("cccccccccccccccccccccccccccccccccccccccc");
        let mut blob_cache = HashMap::new();
        blob_cache.insert(old_a, b"line one\nline two\nline three\n".to_vec());
        blob_cache.insert(old_b, b"totally different\n".to_vec());
        blob_cache.insert(new_c, b"line one\nline two\nline changed\n".to_vec());

        let diff = calculate_tree_diff_with_blobs(
            vec![
                (PathBuf::from("old/a.txt"), old_a),
                (PathBuf::from("old/b.txt"), old_b),
            ],
            vec![(PathBuf::from("new/c.txt"), new_c)],
            &RenameConfig {
                similarity_threshold: 50,
                rename_limit: 1,
                ..RenameConfig::default()
            },
            &blob_cache,
        )
        .unwrap();

        assert!(diff.iter().any(|item| {
            matches!(item, ClDiffFile::Deleted(path, _) if path == &PathBuf::from("old/a.txt"))
        }));
        assert!(diff.iter().any(|item| {
            matches!(item, ClDiffFile::Deleted(path, _) if path == &PathBuf::from("old/b.txt"))
        }));
        assert!(diff.iter().any(|item| {
            matches!(item, ClDiffFile::New(path, _) if path == &PathBuf::from("new/c.txt"))
        }));
        assert!(!diff.iter().any(|item| {
            matches!(
                item,
                ClDiffFile::Renamed(_, _, _, _, _) | ClDiffFile::Moved(_, _, _, _, _)
            )
        }));
    }

    #[test]
    fn inexact_matching_prefers_the_highest_similarity_candidate() {
        let old_best = hash("dddddddddddddddddddddddddddddddddddddddd");
        let old_other = hash("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        let new_hash = hash("ffffffffffffffffffffffffffffffffffffffff");
        let mut blob_cache = HashMap::new();
        blob_cache.insert(old_best, b"alpha\nbeta\ngamma\ndelta\n".to_vec());
        blob_cache.insert(old_other, b"wildly different content\n".to_vec());
        blob_cache.insert(new_hash, b"alpha\nbeta\ngamma\nepsilon\n".to_vec());

        let diff = calculate_tree_diff_with_blobs(
            vec![
                (PathBuf::from("old/one.txt"), old_best),
                (PathBuf::from("old/two.txt"), old_other),
            ],
            vec![(PathBuf::from("new/renamed.txt"), new_hash)],
            &RenameConfig::default(),
            &blob_cache,
        )
        .unwrap();

        assert!(diff.iter().any(|item| {
            matches!(
                item,
                ClDiffFile::Moved(old_path, new_path, _, _, similarity)
                    if old_path == &PathBuf::from("old/one.txt")
                        && new_path == &PathBuf::from("new/renamed.txt")
                        && *similarity >= 50
            )
        }));
    }
}
