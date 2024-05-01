use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use clap::Parser;
use venus::hash::SHA1;
use venus::internal::object::blob::Blob;
use venus::internal::object::commit::Commit;
use venus::internal::object::ObjectTrait;
use venus::internal::object::tree::Tree;
use venus::internal::object::types::ObjectType;
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::internal::index::{Index, IndexEntry};
use crate::utils::object_ext::{BlobExt, CommitExt, TreeExt};
use crate::utils::path_ext::PathExt;
use crate::utils::util;

#[derive(Parser, Debug)]
pub struct RestoreArgs {
    /// files or dir to restore
    #[clap(required = true)]
    pathspec: Vec<String>,
    /// source
    #[clap(long, short)]
    source: Option<String>,
    /// worktree
    #[clap(long, short = 'W')]
    worktree: bool,
    /// staged
    #[clap(long, short = 'S')]
    staged: bool,
}

pub async fn execute(args: RestoreArgs) {
    if !util::check_repo_exist() {
        return;
    }
    let staged = args.staged;
    let mut worktree = args.worktree;
    // If neither option is specified, by default the `working tree` is restored.
    // Specifying `--staged` will only restore the `index`. Specifying both restores both.
    if !staged {
        worktree = true;
    }

    const HEAD: &str = "HEAD"; // prevent misspelling
    let mut source = args.source;
    if source.is_none() && staged {
        // If `--source` not specified, the contents are restored from `HEAD` if `--staged` is given,
        // otherwise from the [index].
        source = Some(HEAD.to_string());
    }

    let storage = util::objects_storage();
    let target_commit: Option<SHA1> = match source {
        None => {
            assert_eq!(staged, false); // pre-processed ↑
            None // Index
        }
        Some(ref src) => { // ref: prevent moving `source`
            if src == HEAD {
                // Default Source
                Head::current_commit().await
            } else if Branch::exists(&src).await {
                // Branch Name, e.g. master
                Branch::current_commit(&src).await
            } else {
                // [Commit Hash, e.g. a1b2c3d4] || [Wrong Branch Name]
                let objs = storage.search(&src);
                // TODO hash can be `commit` or `tree`
                if objs.len() != 1 || !storage.is_object_type(&objs[0], ObjectType::Commit) {
                    None // Wrong Commit Hash
                } else {
                    Some(objs[0])
                }
            }
        }
    };

    // to workdir path
    let target_blobs: Vec<(PathBuf, SHA1)> = {
        // `source` has been pre-process before ↑
        if source.is_none() {
            // only this situation, restore from [Index]
            assert_eq!(staged, false);
            let index = Index::load().unwrap();
            index.tracked_entries(0).into_iter()
                .map(|entry| (PathBuf::from(&entry.name), entry.hash))
                .collect()
        } else {
             // restore from commit hash
            if let Some(commit) = target_commit {
                let tree_id = Commit::load(&commit).tree_id;
                let tree = Tree::load(&tree_id);
                tree.get_plain_items()
            } else {
                let src = source.unwrap();
                if storage.search(&src).len() != 1 {
                    eprintln!("fatal: could not resolve {}", src);
                } else {
                    eprintln!("fatal: reference is not a commit: {}", src);
                }
                return;
            }
        }
    };

    // String to PathBuf
    let paths = args.pathspec.iter().map(PathBuf::from).collect::<Vec<PathBuf>>();
    // restore worktree and staged respectively
    if worktree {
        restore_worktree(&paths, &target_blobs);
    }
    if staged {
        restore_index(&paths, &target_blobs);
    }
}

/// to HashMap
/// - `blobs`: to workdir
fn preprocess_blobs(blobs: &Vec<(PathBuf, SHA1)>) -> HashMap<PathBuf, SHA1> { // TODO maybe can be HashMap<&PathBuf, &SHA1>
    blobs.iter().map(|(path, hash)| (path.clone(), hash.clone())).collect()
}

/// restore a blob to file
/// - `path` : to workdir
fn restore_to_file(hash: &SHA1, path: &PathBuf) {
    let blob = Blob::load(hash);
    let path_abs = util::workdir_to_absolute(path);
    util::write_file(&blob.data, &path_abs).unwrap();
}

/// Get the deleted files in the worktree(vs Index), filtered by `filters`
/// - filters: absolute path or relative path to current dir
/// - target_blobs: to workdir path
fn get_worktree_deleted_files_in_filters(
    filters: &Vec<PathBuf>,
    target_blobs: &HashMap<PathBuf, SHA1>,
) -> HashSet<PathBuf> {
    target_blobs // to workdir
        .iter()
        .filter(|(path, _)| {
            let path = util::workdir_to_absolute(path); // to absolute path
            !path.exists() && path.sub_of_paths(filters) // in filters & target but not in workdir
        })
        .map(|(path, _)| path.clone())
        .collect() // HashSet auto deduplication
}

/// Restore the worktree
/// - `filter`: abs or relative to current (user input)
/// - `target_blobs`: to workdir path
pub fn restore_worktree(filter: &Vec<PathBuf>, target_blobs: &Vec<(PathBuf, SHA1)>) {
    let target_blobs = preprocess_blobs(target_blobs);
    let deleted_files = get_worktree_deleted_files_in_filters(filter, &target_blobs);

    { // validate input pathspec(filter)
        for path in filter { // abs or relative to cur
            if !path.exists() { //TODO bug problem: 路径设计大问题，全部统一为to workdir
                if !target_blobs.iter().any(|(p, _)| util::is_sub_path(&p.workdir_to_absolute(), path)) {
                    // not in target_blobs & worktree, illegal path
                    eprintln!("fatal: pathspec '{}' did not match any files", path.display());
                    return; // once fatal occurs, nothing should be done
                }
            }
        }
    }

    // to workdir path
    let mut file_paths = util::integrate_pathspec(filter);
    file_paths.extend(deleted_files);

    let index = Index::load().unwrap();
    for path_wd in &file_paths {
        let path_abs = util::workdir_to_absolute(path_wd);
        if !path_abs.exists() {
            // file not exist, deleted or illegal
            if target_blobs.contains_key(path_wd) {
                // file in target_blobs (deleted), need to restore
                restore_to_file(&target_blobs[path_wd], &path_wd);
            } else {
                // not in target_commit and workdir (illegal path), user input
                unreachable!("It should be checked before");
            }
        } else {
            // file exists
            let path_wd_str = path_wd.to_string_or_panic();
            let hash = util::calc_file_hash(&path_abs).unwrap();
            if target_blobs.contains_key(path_wd) {
                // both in target & worktree: 1. modified 2. same
                if hash != target_blobs[path_wd] {
                    // modified
                    restore_to_file(&target_blobs[path_wd], &path_wd);
                } // else: same, keep
            } else {
                // not in target but in worktree: New file
                if index.tracked(&path_wd_str, 0) {
                    // tracked, need to delete
                    fs::remove_file(&path_abs).unwrap();
                    util::clear_empty_dir(&path_abs); // clean empty dir in cascade
                } // else: untracked, keep
            }
        }
    }
}

/// Get the deleted files in the `index`(vs target_blobs), filtered by `filters`
fn get_index_deleted_files_in_filters(
    index: &Index,
    filters: &Vec<PathBuf>,
    target_blobs: &HashMap<PathBuf, SHA1>,
) -> HashSet<PathBuf> {
    target_blobs
        .iter()
        .filter(|(path_wd, _)| { // to workdir
            let path_abs = util::workdir_to_absolute(path_wd); // to absolute path
            !index.tracked(&path_wd.to_string_or_panic(), 0) && util::is_sub_of_paths(path_abs, filters)
        })
        .map(|(path, _)| path.clone())
        .collect() // HashSet auto deduplication
}

pub fn restore_index(filter: &Vec<PathBuf>, target_blobs: &Vec<(PathBuf, SHA1)>) {
    let target_blobs = preprocess_blobs(target_blobs);

    let mut index = Index::load().unwrap();
    let deleted_files_index = get_index_deleted_files_in_filters(&index, filter, &target_blobs);

    let mut file_paths = util::filter_to_fit_paths(&index.tracked_files(), filter);
    file_paths.extend(deleted_files_index); // maybe we should not integrate them rater than deal separately

    for path in &file_paths { // to workdir
        let path_str = path.to_string_or_panic();
        let hash = target_blobs[path];
        if !index.tracked(&path_str, 0) {
            // file not exist in index
            if target_blobs.contains_key(path) {
                // file in target_blobs (deleted), need to restore
                let blob = Blob::load(&hash);
                index.add(IndexEntry::new_from_blob(path_str, hash, blob.data.len() as u32));
            } else {
                eprintln!("fatal: pathspec '{}' did not match any files", path.display());
                continue; // TODO once fatal occurs, nothing should be done
            }
        } else {
            // file exists in index: 1. modified 2. same 3. need to deleted
            if target_blobs.contains_key(path) {
                if !index.verify_hash(&path_str, 0, &hash) {
                    // modified
                    let blob = Blob::load(&hash);
                    index.update(IndexEntry::new_from_blob(path_str, hash, blob.data.len() as u32));
                } // else: same, keep
            } else {
                // not in target but in index: need to delete
                index.remove(&path_str, 0); // TODO all stages
            }
        }
    }
    index.save().unwrap(); // DO NOT forget to save
}