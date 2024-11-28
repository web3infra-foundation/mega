use std::{
    collections::HashMap,
    fmt,
    io::{self, Write},
    path::PathBuf,
};

use clap::Parser;
use mercury::{
    hash::SHA1,
    internal::{
        index::Index,
        object::{blob::Blob, commit::Commit, tree::Tree, types::ObjectType},
        pack::utils::calculate_object_hash,
    },
};
use similar;

use crate::{
    command::{
        get_target_commit, load_object,
        status::{self, changes_to_be_committed},
    },
    internal::head::Head,
    utils::{object_ext::TreeExt, path, util},
};

#[cfg(unix)]
use std::process::{Command, Stdio};

use crate::utils::path_ext::PathExt;

#[derive(Parser, Debug)]
pub struct DiffArgs {
    #[clap(long, help = "Old commit, defaults is staged or HEAD")]
    pub old: Option<String>,

    #[clap(long, help = "New commit, default is working directory")]
    #[clap(requires = "old", group = "op_new")]
    pub new: Option<String>,

    #[clap(long, help = "use stage as new commit")]
    #[clap(group = "op_new")]
    pub staged: bool,

    #[clap(help = "Files to compare")]
    pathspec: Vec<String>,

    #[clap(long)]
    pub output: Option<String>,
}

pub async fn execute(args: DiffArgs) {
    if !util::check_repo_exist() {
        return;
    }
    tracing::debug!("diff args: {:?}", args);
    let index = Index::load(path::index()).unwrap();
    #[cfg(unix)]
    let mut child = Command::new("less")
        .arg("-R")
        .arg("-F")
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    let mut w = match args.output {
        Some(ref path) => {
            let file = std::fs::File::create(path)
                .map_err(|e| {
                    eprintln!(
                        "fatal: could not open to file '{}' for writing: {}",
                        path, e
                    );
                })
                .unwrap();
            Some(file)
        }
        None => None,
    };

    let old_blobs = match args.old {
        Some(ref source) => match get_target_commit(source).await {
            Ok(commit_hash) => get_commit_blobs(&commit_hash).await,
            Err(e) => {
                eprintln!("fatal: {}, can't use as diff old source", e);
                return;
            }
        },
        None => {
            // if the staged is not empty, use it as old commit. Otherwise, use HEAD
            if status::changes_to_be_committed().await.is_empty() {
                let commit_hash = Head::current_commit().await.unwrap();
                get_commit_blobs(&commit_hash).await
            } else {
                let changes = changes_to_be_committed().await;
                // diff didn't show untracked or deleted files
                get_files_blobs(&changes.modified)
            }
        }
    };

    let new_blobs = match args.new {
        Some(ref source) => match get_target_commit(source).await {
            Ok(commit_hash) => get_commit_blobs(&commit_hash).await,
            Err(e) => {
                eprintln!("fatal: {}, can't use as diff new source", e);
                return;
            }
        },
        None => {
            let files = if args.staged {
                // use staged as new commit
                index.tracked_files()
            } else {
                // use working directory as new commit
                util::list_workdir_files().unwrap()
            };
            get_files_blobs(&files)
        }
    };

    // use pathspec to filter files
    let paths: Vec<PathBuf> = args
        .pathspec
        .iter()
        .map(|s| {
            util::to_workdir_path(s)
        })
        .collect();

    let mut buf: Vec<u8> = Vec::new();
    // filter files, cross old and new files, and pathspec
    diff(old_blobs, new_blobs, paths.into_iter().collect(), &mut buf).await;

    match w {
        Some(ref mut file) => {
            file.write_all(&buf).unwrap();
        }
        None => {
            #[cfg(unix)]
            {
                let stdin = child.stdin.as_mut().unwrap();
                stdin.write_all(&buf).unwrap();
                child.wait().unwrap();
            }
            #[cfg(not(unix))]
            {
                io::stdout().write_all(&buf).unwrap();
            }
        }
    }
}

pub async fn diff(
    old_blobs: Vec<(PathBuf, SHA1)>,
    new_blobs: Vec<(PathBuf, SHA1)>,
    filter: Vec<PathBuf>,
    w: &mut dyn io::Write,
) {
    let old_blobs: HashMap<PathBuf, SHA1> = old_blobs.into_iter().collect();
    let read_content = |file: &PathBuf, hash: &SHA1| {
        // read content from blob or file
        match load_object::<Blob>(hash) {
            Ok(blob) => String::from_utf8(blob.data).unwrap(),
            Err(_) => {
                let file = util::workdir_to_absolute(file);
                std::fs::read_to_string(&file)
                    .map_err(|e| {
                        eprintln!("fatal: could not read file '{}': {}", file.display(), e);
                    })
                    .unwrap()
            }
        }
    };
    // filter files, cross old and new files, and pathspec
    for (new_file, new_hash) in new_blobs {
        // if new_file did't start with any path in filter, skip it
        if !filter.is_empty() && !filter.iter().any(|path| new_file.sub_of(path)) {
            continue;
        }
        match old_blobs.get(&new_file) {
            Some(old_hash) => {
                if old_hash == &new_hash {
                    continue;
                }
                let old_content = read_content(&new_file, old_hash);
                let new_content = read_content(&new_file, &new_hash);
                writeln!(
                    w,
                    "diff --git a/{} b/{}",
                    new_file.display(),
                    new_file.display() // files name is always the same, current did't support rename
                )
                .unwrap();
                writeln!(
                    w,
                    "index {}..{}",
                    &old_hash.to_plain_str()[0..8],
                    &new_hash.to_plain_str()[0..8]
                )
                .unwrap();

                diff_result(&old_content, &new_content, w);
            }
            None => {
                continue;
            }
        }
    }
}

async fn get_commit_blobs(commit_hash: &SHA1) -> Vec<(PathBuf, SHA1)> {
    let commit = load_object::<Commit>(commit_hash).unwrap();
    let tree = load_object::<Tree>(&commit.tree_id).unwrap();
    tree.get_plain_items()
}

// diff need to print hash even if the file is not added
fn get_files_blobs(files: &[PathBuf]) -> Vec<(PathBuf, SHA1)> {
    files
        .iter()
        .map(|p| {
            let path = util::workdir_to_absolute(p);
            let data = std::fs::read(&path).unwrap();
            (p.to_owned(), calculate_object_hash(ObjectType::Blob, &data))
        })
        .collect()
}

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

fn diff_result(old: &str, new: &str, w: &mut dyn io::Write) {
    let diff = similar::TextDiff::from_lines(old, new);
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    similar::ChangeTag::Delete => "-",
                    similar::ChangeTag::Insert => "+",
                    similar::ChangeTag::Equal => " ",
                };
                write!(
                    w,
                    "{}{} |{}",
                    Line(change.old_index()),
                    Line(change.new_index()),
                    sign
                )
                .unwrap();
                write!(w, "{}", change.value()).unwrap();
                if change.missing_newline() {
                    writeln!(w).unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_args() {
        {
            let args = DiffArgs::try_parse_from(["diff", "--old", "old", "--new", "new", "paths"]);
            assert!(args.is_ok());
            let args = args.unwrap();
            // println!("{:?}", args);
            assert_eq!(args.old, Some("old".to_string()));
            assert_eq!(args.new, Some("new".to_string()));
            assert_eq!(args.pathspec, vec!["paths".to_string()]);
        }
        {
            // --staged didn't require --old
            let args =
                DiffArgs::try_parse_from(["diff", "--staged", "pathspec", "--output", "output"]);
            let args = args.unwrap();
            assert_eq!(args.old, None);
            assert!(args.staged);
        }
        {
            // --staged conflicts with --new
            let args = DiffArgs::try_parse_from([
                "diff", "--old", "old", "--new", "new", "--staged", "paths",
            ]);
            assert!(args.is_err());
            assert!(args.err().unwrap().kind() == clap::error::ErrorKind::ArgumentConflict);
        }
        {
            // --new requires --old
            let args = DiffArgs::try_parse_from([
                "diff", "--new", "new", "pathspec", "--output", "output",
            ]);
            assert!(args.is_err());
            assert!(args.err().unwrap().kind() == clap::error::ErrorKind::MissingRequiredArgument);
        }
    }

    #[test]
    fn test_diff_result() {
        let old = "Hello World\nThis is the second line.\nThis is the third.";
        let new = "Hallo Welt\nThis is the second line.\nThis is life.\nMoar and more";
        let mut buf = Vec::new();
        diff_result(old, new, &mut buf);
        let result = String::from_utf8(buf).unwrap();
        println!("{}", result);
    }
}
