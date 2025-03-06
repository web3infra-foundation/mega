use std::fs;
use std::path::PathBuf;

use clap::Parser;
use colored::Colorize;

use mercury::errors::GitError;

use crate::utils::path_ext::PathExt;
use crate::utils::{path, util};
use mercury::internal::index::Index;

#[derive(Parser, Debug)]
pub struct RemoveArgs {
    /// file or dir to remove
    pathspec: Vec<String>,
    /// whether to remove from index
    #[clap(long)]
    cached: bool,
    /// indicate recursive remove dir
    #[clap(short, long)]
    recursive: bool,
}

pub fn execute(args: RemoveArgs) -> Result<(), GitError> {
    if !util::check_repo_exist() {
        return Ok(());
    }
    let idx_file = path::index();
    let mut index = Index::load(&idx_file)?;
    // check if pathspec is all in index
    if !validate_pathspec(&args.pathspec, &index) {
        return Ok(());
    }
    let dirs = get_dirs(&args.pathspec, &index);
    if !dirs.is_empty() && !args.recursive {
        println!(
            "fatal: not removing '{}' recursively without -r",
            dirs[0].bright_blue()
        ); // Git print first
        return Ok(());
    }

    for path_str in args.pathspec.iter() {
        let path = PathBuf::from(path_str);
        let path_wd = path.to_workdir().to_string_or_panic();
        if dirs.contains(path_str) {
            // dir
            let removed = index.remove_dir_files(&path_wd);
            for file in removed.iter() {
                // to workdir
                println!("rm '{}'", file.bright_green());
            }
            if !args.cached {
                fs::remove_dir_all(&path)?;
            }
        } else {
            // file
            index.remove(&path_wd, 0);
            println!("rm '{}'", path_wd.bright_green());
            if !args.cached {
                fs::remove_file(&path)?;
            }
        }
    }
    index.save(&idx_file)?;
    Ok(())
}

/// check if pathspec is all valid(in index)
/// - if path is a dir, check if any file in the dir is in index
fn validate_pathspec(pathspec: &[String], index: &Index) -> bool {
    if pathspec.is_empty() {
        println!("fatal: No pathspec was given. Which files should I remove?");
        return false;
    }
    for path_str in pathspec.iter() {
        let path = PathBuf::from(path_str);
        let path_wd = path.to_workdir().to_string_or_panic();
        if !index.tracked(&path_wd, 0) {
            // not tracked, but path may be a directory
            // check if any tracked file in the directory
            if !index.contains_dir_file(&path_wd) {
                println!("fatal: pathspec '{}' did not match any files", path_str);
                return false;
            }
        }
    }
    true
}

/// run after `validate_pathspec`
fn get_dirs(pathspec: &[String], index: &Index) -> Vec<String> {
    let mut dirs = Vec::new();
    for path_str in pathspec.iter() {
        let path = PathBuf::from(path_str);
        let path_wd = path.to_workdir().to_string_or_panic();
        // valid but not tracked, means a dir
        if !index.tracked(&path_wd, 0) {
            dirs.push(path_str.clone());
        }
    }
    dirs
}
