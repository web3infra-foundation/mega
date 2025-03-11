use crate::command::status;
use crate::utils::object_ext::BlobExt;
use clap::Parser;
use mercury::internal::index::{Index, IndexEntry};
use mercury::internal::object::blob::Blob;
use std::path::{Path, PathBuf};

use crate::utils::{lfs, path, util};

#[derive(Parser, Debug)]
pub struct AddArgs {
    /// pathspec... files & dir to add content from.
    #[clap(required = false)]
    pub pathspec: Vec<String>,

    /// Update the index not only where the working tree has a file matching pathspec but also where the index already has an entry. This adds, modifies, and removes index entries to match the working tree.
    ///
    /// If no pathspec is given when -A option is used, all files in the entire working tree are updated
    #[clap(short = 'A', long, group = "mode")]
    pub all: bool,

    /// Update the index just where it already has an entry matching **pathspec**.
    /// This removes as well as modifies index entries to match the working tree, but adds no new files.
    #[clap(short, long, group = "mode")]
    pub update: bool,

    /// more detailed output
    #[clap(short, long)]
    pub verbose: bool,
}

pub async fn execute(args: AddArgs) {
    // TODO .gitignore
    if !util::check_repo_exist() {
        return;
    }

    // `String` to `PathBuf`
    let mut paths: Vec<PathBuf> = args.pathspec.iter().map(PathBuf::from).collect();
    if args.pathspec.is_empty() {
        if !args.all && !args.update {
            println!("Nothing specified, nothing added.");
            return;
        } else {
            // add all files in the entire working tree
            paths.push(util::working_dir());
        } // '-A' and '-u' cannot be used together
    }

    // index vs worktree
    let mut changes = status::changes_to_be_staged(); // to workdir
                                                      // filter paths to fit `pathspec` that user inputs
    changes.new = util::filter_to_fit_paths(&changes.new, &paths);
    // if `--all` & <pathspec> is given, it will update `index` as well, so no need to filter `deleted` & `modified`
    if args.pathspec.is_empty() || !args.all {
        changes.modified = util::filter_to_fit_paths(&changes.modified, &paths);
        changes.deleted = util::filter_to_fit_paths(&changes.deleted, &paths);
    }

    let mut files = changes.modified;
    files.extend(changes.deleted);
    // `--update` only operates on tracked files, not including `new` files
    if !args.update {
        files.extend(changes.new);
    }

    let index_file = path::index();
    let mut index = Index::load(&index_file).unwrap();
    for file in &files {
        add_a_file(file, &mut index, args.verbose).await;
    }
    index.save(&index_file).unwrap();
}

/// `file` path must relative to the working directory
async fn add_a_file(file: &Path, index: &mut Index, verbose: bool) {
    let workdir = util::working_dir();
    if !util::is_sub_path(file, &workdir) {
        // file is not in the working directory
        // TODO check this earlier, once fatal occurs, nothing should be done
        println!(
            "fatal: '{}' is outside workdir at '{}'",
            file.display(),
            workdir.display()
        );
        return;
    }
    if util::is_sub_path(file, util::storage_path()) {
        // file is in `.libra`
        // Git won't print this
        println!(
            "warning: '{}' is inside '{}' repo, which will be ignored by `add`",
            file.display(),
            util::ROOT_DIR
        );
        return;
    }

    let file_abs = util::workdir_to_absolute(file);
    let file_str = file.to_str().unwrap();
    if !file_abs.exists() {
        if index.tracked(file_str, 0) {
            // file is removed
            index.remove(file_str, 0);
            if verbose {
                println!("removed: {}", file_str);
            }
        } else {
            // FIXME: unreachable code! This situation is not included in `status::changes_to_be_staged()`
            // FIXME: should check files in original input paths
            // TODO do this check earlier, once fatal occurs, nothing should be done
            // file is not tracked && not exists, which means wrong pathspec
            println!(
                "fatal: pathspec '{}' did not match any files",
                file.display()
            );
        }
    } else {
        // file exists
        if !index.tracked(file_str, 0) {
            // file is not tracked
            let blob = gen_blob_from_file(&file_abs);
            blob.save();
            index.add(IndexEntry::new_from_file(file, blob.id, &workdir).unwrap());
            if verbose {
                println!("add(new): {}", file.display());
            }
        } else {
            // file is tracked, maybe modified
            if index.is_modified(file_str, 0, &workdir) {
                // file is modified(meta), but content may not change
                let blob = gen_blob_from_file(&file_abs);
                if !index.verify_hash(file_str, 0, &blob.id) {
                    // content is changed
                    blob.save();
                    index.update(IndexEntry::new_from_file(file, blob.id, &workdir).unwrap());
                    if verbose {
                        println!("add(modified): {}", file.display());
                    }
                }
            }
        }
    }
}

/// Generate a `Blob` from a file
/// - if the file is tracked by LFS, generate a `Blob` with pointer file
fn gen_blob_from_file(path: impl AsRef<Path>) -> Blob {
    if lfs::is_lfs_tracked(&path) {
        Blob::from_lfs_file(&path)
    } else {
        Blob::from_file(&path)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic]
    fn test_args_parse_update_conflict_with_all() {
        AddArgs::try_parse_from(["test", "-A", "-u"]).unwrap();
    }
}
