use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
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
    /// Old commit, default is HEAD
    #[clap(long, value_name = "COMMIT")]
    pub old: Option<String>,

    /// New commit, default is working directory
    #[clap(long, value_name = "COMMIT")]
    #[clap(requires = "old", group = "op_new")]
    pub new: Option<String>,

    /// Use stage as new commit. This option is conflict with --new.
    #[clap(long)]
    #[clap(group = "op_new")]
    pub staged: bool,

    #[clap(help = "Files to compare")]
    pathspec: Vec<String>,

    /// choose the exact diff algorithm default value is histogram
    /// support myers and myersMinimal
    #[clap(long, default_value = "histogram", value_parser=["histogram", "myers", "myersMinimal"])]
    pub algorithm: Option<String>,

    // Print the result to file
    #[clap(long, value_name = "FILENAME")]
    pub output: Option<String>,
}

pub async fn execute(args: DiffArgs) {
    if !util::check_repo_exist() {
        return;
    }
    tracing::debug!("diff args: {:?}", args);
    let index = Index::load(path::index()).unwrap();

    let mut w = match args.output {
        Some(ref path) => {
            let file = std::fs::File::create(path)
                .map_err(|e| {
                    eprintln!("fatal: could not open to file '{path}' for writing: {e}");
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
                eprintln!("fatal: {e}, can't use as diff old source");
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
                eprintln!("fatal: {e}, can't use as diff new source");
                return;
            }
        },
        None => {
            let files = if args.staged {
                // use staged as new commit
                index.tracked_files()
            } else {
                // use working directory as new commit
                // NOTE: git didn't show diff for untracked files, but we do
                util::list_workdir_files().unwrap()
            };
            get_files_blobs(&files)
        }
    };

    // use pathspec to filter files
    let paths: Vec<PathBuf> = args.pathspec.iter().map(util::to_workdir_path).collect();

    let mut buf: Vec<u8> = Vec::new();
    // filter files, cross old and new files, and pathspec
    diff(
        old_blobs,
        new_blobs,
        args.algorithm.unwrap_or_default(),
        paths.into_iter().collect(),
        &mut buf,
    )
    .await;

    match w {
        Some(ref mut file) => {
            file.write_all(&buf).unwrap();
        }
        None => {
            #[cfg(unix)]
            {
                #[cfg(unix)]
                let mut child = Command::new("less")
                    .arg("-R")
                    .arg("-F")
                    .stdin(Stdio::piped())
                    .spawn()
                    .expect("failed to execute process");
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
    algorithm: String,
    filter: Vec<PathBuf>,
    w: &mut dyn io::Write,
) {
    let old_blobs: HashMap<PathBuf, SHA1> = old_blobs.into_iter().collect();
    let new_blobs: HashMap<PathBuf, SHA1> = new_blobs.into_iter().collect();
    // unison set
    let union_files: HashSet<PathBuf> = old_blobs.keys().chain(new_blobs.keys()).cloned().collect();
    tracing::debug!(
        "old blobs {:?}, new blobs {:?}, union files {:?}",
        old_blobs.len(),
        new_blobs.len(),
        union_files.len()
    );

    let read_content = |file: &PathBuf, hash: &SHA1| {
        // read content from blob or file
        match load_object::<Blob>(hash) {
            Ok(blob) => blob.data,
            Err(_) => {
                let file = util::workdir_to_absolute(file);
                std::fs::read(&file)
                    .map_err(|e| {
                        eprintln!("fatal: could not read file '{}': {}", file.display(), e);
                    })
                    .unwrap()
            }
        }
    };

    // filter files, cross old and new files, and pathspec
    for file in union_files {
        // if new_file did't start with any path in filter, skip it
        if !filter.is_empty() && !filter.iter().any(|path| file.sub_of(path)) {
            continue;
        }

        let new_hash = new_blobs.get(&file);
        let old_hash = old_blobs.get(&file);
        if new_hash == old_hash {
            continue;
        }

        let old_content = match old_hash.as_ref() {
            Some(hash) => read_content(&file, hash),
            None => Vec::new(),
        };
        let new_content = match new_hash.as_ref() {
            Some(hash) => read_content(&file, hash),
            None => Vec::new(),
        };

        writeln!(
            w,
            "diff --git a/{} b/{}",
            file.display(),
            file.display() // files name is always the same, current did't support rename
        )
        .unwrap();

        if old_hash.is_none() {
            writeln!(w, "new file mode 100644").unwrap();
        } else if new_hash.is_none() {
            writeln!(w, "deleted file mode 100644").unwrap();
        }

        let old_index = old_hash.map_or("0000000".to_string(), |h| h.to_string()[0..8].to_string());
        let new_index = new_hash.map_or("0000000".to_string(), |h| h.to_string()[0..8].to_string());
        writeln!(w, "index {old_index}..{new_index}").unwrap();
        // check is the content is valid utf-8 or maybe binary
        let old_type = infer::get(&old_content);
        let new_type = infer::get(&new_content);
        match (
            String::from_utf8(old_content),
            String::from_utf8(new_content),
        ) {
            (Ok(old_text), Ok(new_text)) => {
                let (old_prefix, new_prefix) = if old_text.is_empty() {
                    // New file
                    (
                        "/dev/null".to_string(),
                        format!("b/{}", file_display(&file, new_hash, new_type)),
                    )
                } else if new_text.is_empty() {
                    // Remove file
                    (
                        format!("a/{}", file_display(&file, old_hash, old_type)),
                        "/dev/null".to_string(),
                    )
                } else {
                    // Update file
                    (
                        format!("a/{}", file_display(&file, old_hash, old_type)),
                        format!("b/{}", file_display(&file, new_hash, new_type)),
                    )
                };
                writeln!(w, "--- {old_prefix}").unwrap();
                writeln!(w, "+++ {new_prefix}").unwrap();
                imara_diff_result(&old_text, &new_text, algorithm.as_str(), w);
            }
            _ => {
                // TODO: Handle non-UTF-8 data as binary for now; consider optimization in the future.
                writeln!(
                    w,
                    "Binary files a/{} and b/{} differ",
                    file_display(&file, old_hash, old_type),
                    file_display(&file, new_hash, new_type)
                )
                .unwrap();
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
    let working_dir = util::working_dir();
    files
        .iter()
        .filter(|&p| {
            let path = util::workdir_to_absolute(p);
            !util::check_gitignore(&working_dir, &path)
        })
        .map(|p| {
            let path = util::workdir_to_absolute(p);
            let data = std::fs::read(&path).unwrap();
            (p.to_owned(), calculate_object_hash(ObjectType::Blob, &data))
        })
        .collect()
}

// display file with type
fn file_display(file: &Path, hash: Option<&SHA1>, file_type: Option<infer::Type>) -> String {
    let file_name = match hash {
        Some(_) => file.display().to_string(),
        None => "dev/null".to_string(),
    };

    if let Some(file_type) = file_type {
        // Check if the file type is displayable in browser, like image, audio, video, etc.
        if matches!(
            file_type.matcher_type(),
            infer::MatcherType::Audio | infer::MatcherType::Video | infer::MatcherType::Image
        ) {
            return format!("{} ({})", file_name, file_type.mime_type()).to_string();
        }
    }
    file_name
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

#[allow(dead_code)]
fn similar_diff_result(old: &str, new: &str, w: &mut dyn io::Write) {
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

fn imara_diff_result(old: &str, new: &str, algorithm: &str, w: &mut dyn io::Write) {
    let input = InternedInput::new(old, new);

    let algo = match algorithm {
        "myers" => Algorithm::Myers,
        "myersMinimal" => Algorithm::MyersMinimal,
        // default is the histogram algo
        _ => Algorithm::Histogram,
    };
    tracing::debug!("libra [diff]: choose the algorithm: {:?}", algo);

    let mut diff = Diff::compute(algo, &input);

    // did the postprocess_lines
    diff.postprocess_lines(&input);

    let result = diff
        .unified_diff(
            &BasicLineDiffPrinter(&input.interner),
            UnifiedDiffConfig::default(),
            &input,
        )
        .to_string();

    write!(w, "{result}").unwrap();
}

#[cfg(test)]
mod test {
    use crate::utils::test;
    use serial_test::serial;
    use std::fs;
    use std::time::Instant;
    use tempfile::tempdir;

    use super::*;
    #[test]
    /// Tests command line argument parsing for the diff command with various parameter combinations.
    /// Verifies parameter requirements, conflicts and default values are handled correctly.
    fn test_args() {
        {
            let args = DiffArgs::try_parse_from(["diff", "--old", "old", "--new", "new", "paths"]);
            assert!(args.is_ok());
            let args = args.unwrap();
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
        {
            // --algorithm arg
            let args = DiffArgs::try_parse_from([
                "diff",
                "--old",
                "old",
                "--new",
                "new",
                "--algorithm",
                "myers",
                "target paths",
            ])
            .unwrap();
            assert_eq!(args.algorithm, Some("myers".to_string()));
        }
        {
            // --algorithm arg with default value
            let args = DiffArgs::try_parse_from(["diff", "--old", "old", "target paths"]).unwrap();
            assert_eq!(args.algorithm, Some("histogram".to_string()));
        }
    }

    #[test]
    /// Tests the functionality of the `similar_diff_result` function.
    /// Verifies that it correctly generates a diff between two text inputs.
    fn test_similar_diff_result() {
        let old = "Hello World\nThis is the second line.\nThis is the third.";
        let new = "Hallo Welt\nThis is the second line.\nThis is life.\nMoar and more";
        let mut buf = Vec::new();
        similar_diff_result(old, new, &mut buf);
        let result = String::from_utf8(buf).unwrap();
        println!("{result}");
    }

    #[tokio::test]
    #[serial]
    /// Tests that the get_files_blobs function properly respects .libraignore patterns.
    /// Verifies ignored files are correctly excluded from the blob collection process.
    async fn test_get_files_blob_gitignore() {
        let temp_path = tempdir().unwrap();
        test::setup_with_new_libra_in(temp_path.path()).await;
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        let mut gitignore_file = fs::File::create(".libraignore").unwrap();
        gitignore_file.write_all(b"should_ignore").unwrap();

        fs::File::create("should_ignore").unwrap();
        fs::File::create("not_ignore").unwrap();

        let blob = get_files_blobs(&[PathBuf::from("should_ignore"), PathBuf::from("not_ignore")]);
        assert_eq!(blob.len(), 1);
        assert_eq!(blob[0].0, PathBuf::from("not_ignore"));
    }

    #[test]
    fn test_diff_algorithms_correctness_and_efficiency() {
        let old = r#"function foo() {
    if (condition) {
        doSomething();
        doSomethingElse();
        andAnotherThing();
    } else {
        alternative();
    }
}"#;

        let new = r#"function foo() {
    if (condition) {
        // Added comment
        doSomething();
        // Modified this line
        modifiedSomethingElse();
        andAnotherThing();
    } else {
        alternative();
    }

    // Added new block
    addedNewFunctionality();
}"#;
        let mut outputs = Vec::new();

        let algos = ["histogram", "myers", "myersMinimal"];

        // test the different algo benchmark
        for algo in algos {
            let mut buf = Vec::new();
            let start = Instant::now();
            imara_diff_result(old, new, algo, &mut buf);
            let elapse = start.elapsed();
            let ouput = String::from_utf8(buf).expect("Invalid UTF-8 in diff ouput");

            println!("libra diff algorithm: {algo:?} Spend Time: {elapse:?}");
            assert!(
                !ouput.is_empty(),
                "libra diff algorithm: {algo} produce a empty output"
            );
            assert!(
                ouput.contains("@@"),
                "libra diff algorithm: {algo}, ouput missing diff markers"
            );

            outputs.push((algo, ouput));
        }

        // check the line counter difference
        for (algo, output) in outputs {
            let plus_line = output.lines().filter(|line| line.starts_with("+")).count();
            let minus_line = output.lines().filter(|line| line.starts_with("-")).count();
            assert_eq!(
                plus_line, 6,
                "libra diff algorithm {algo}, expect plus_line: 6, got {plus_line} "
            );
            assert_eq!(
                minus_line, 1,
                "libra diff algorithm {algo}, expect minus_line: 1, got {minus_line} "
            );
        }
    }
}
