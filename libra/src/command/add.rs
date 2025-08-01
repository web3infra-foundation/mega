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

    /// Refresh index entries for all files currently in the index.
    ///
    /// This updates only the metadata (e.g. file stat information such as
    /// timestamps, file size, etc.) of existing index entries to match
    /// the working tree, without adding new files or removing entries.
    #[clap(long, group = "mode")]
    pub refresh: bool,

    /// more detailed output
    #[clap(short, long)]
    pub verbose: bool,

    /// dry run
    #[clap(short, long)]
    pub dry_run: bool,

    /// ignore errors
    #[clap(long)]
    pub ignore_errors: bool,
}

pub async fn execute(args: AddArgs) {
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

    if args.refresh {
        let index_path = path::index();
        let index = Index::load(&index_path).unwrap();

        let files: Vec<PathBuf> = changes
            .modified
            .into_iter()
            .filter(|p| {
                let s = p
                    .to_str()
                    .unwrap_or_else(|| panic!("path {:?} is not valid UTF-8", p.display()));
                index.tracked(s, 0)
            })
            .collect();

        // check for dry_run
        if args.dry_run {
            for file in &files {
                println!("refresh: {}", file.display())
            }
        }

        let index_file = path::index();
        let mut index = Index::load(&index_file).unwrap();
        for file in &files {
            if index
                .refresh(file, &util::working_dir())
                .unwrap_or_else(|_| panic!("error refreshing {}", file.display()))
                && args.verbose
            {
                println!("refreshed: {}", file.display());
            }
        }

        index.save(&index_file).unwrap();
        return;
    }

    let mut files = changes.modified;
    files.extend(changes.deleted);
    // `--update` only operates on tracked files, not including `new` files
    if !args.update {
        files.extend(changes.new);
    }

    if args.dry_run {
        // dry run
        for file in &files {
            println!("add: {}", file.display());
        }
        return;
    }

    let index_file = path::index();
    let mut index = Index::load(&index_file).unwrap();
    for file in &files {
        match add_a_file(file, &mut index, args.verbose).await {
            Ok(_) => {}
            Err(e) => {
                println!("{e}");
                if !args.ignore_errors {
                    // if `--ignore-errors` is not set, stop on first error
                    return;
                }
            }
        }
    }
    index.save(&index_file).unwrap();
}

/// `file` path must relative to the working directory
async fn add_a_file(file: &Path, index: &mut Index, verbose: bool) -> Result<(), String> {
    let workdir = util::working_dir();
    if !util::is_sub_path(file, &workdir) {
        // file is not in the working directory
        // TODO check this earlier, once fatal occurs, nothing should be done
        return Err(format!(
            "fatal: '{}' is outside workdir at '{}'",
            file.display(),
            workdir.display()
        ));
    }
    if util::is_sub_path(file, util::storage_path()) {
        // file is in `.libra`
        // Git won't print this
        return Err(format!(
            "fatal: '{}' is inside '{}' repo, which will be ignored by `add`",
            file.display(),
            util::ROOT_DIR
        ));
    }

    let file_abs = util::workdir_to_absolute(file);
    let file_str = file.to_str().unwrap();
    let file_status = check_file_status(file, index);
    match file_status {
        FileStatus::New => {
            let blob = gen_blob_from_file(&file_abs);
            blob.save();
            index.add(IndexEntry::new_from_file(file, blob.id, &workdir).unwrap());
            if verbose {
                println!("add(new): {}", file.display());
            }
        }
        FileStatus::Modified => {
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
        FileStatus::Deleted => {
            index.remove(file_str, 0);
            if verbose {
                println!("removed: {file_str}");
            }
        }
        FileStatus::NotFound => {
            return Err(format!(
                "fatal: pathspec '{}' did not match any files",
                file.display(),
            ));
        }
    }
    Ok(())
}

enum FileStatus {
    /// file is new
    New,
    /// file is modified
    Modified,
    /// file is deleted
    Deleted,
    /// file is not tracked
    NotFound,
}

fn check_file_status(file: &Path, index: &Index) -> FileStatus {
    let file_str = file.to_str().unwrap();
    if !file.exists() {
        if index.tracked(file_str, 0) {
            FileStatus::Deleted
        } else {
            FileStatus::NotFound
        }
    } else if !index.tracked(file_str, 0) {
        FileStatus::New
    } else if index.is_modified(file_str, 0, &util::working_dir()) {
        FileStatus::Modified
    } else {
        FileStatus::NotFound
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
    fn test_args_conflict_with_refresh() {
        // "--refresh" cannot be combined with "-A", "--refresh" or "-u"
        assert!(AddArgs::try_parse_from(["test", "-A", "--refresh"]).is_err());
        assert!(AddArgs::try_parse_from(["test", "-u", "--refresh"]).is_err());
        assert!(AddArgs::try_parse_from(["test", "-A", "-u", "--refresh"]).is_err());
    }
}
