use std::cmp::min;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;
use colored::Colorize;

#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::process::{Command, Stdio};

use crate::command::load_object;
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use crate::utils::object_ext::TreeExt;
use crate::utils::util;
use common::utils::parse_commit_msg;

use mercury::hash::SHA1;
use mercury::internal::object::{blob::Blob, commit::Commit, tree::Tree};
use neptune::Diff;

/// Command line arguments for `log`
#[derive(Parser, Debug)]
pub struct LogArgs {
    /// Limit the number of output
    #[clap(short, long)]
    pub number: Option<usize>,

    /// Shorthand for --pretty=oneline --abbrev-commit
    #[clap(long)]
    pub oneline: bool,

    /// Show diffs for each commit (like git -p)
    #[clap(short = 'p', long = "patch")]
    pub patch: bool,

    /// Files to limit diff output (only used with -p)
    #[clap(requires = "patch", value_name = "PATHS")]
    pathspec: Vec<String>,
}

/// Get all reachable commits from the given commit hash
/// **didn't consider the order of the commits**
pub async fn get_reachable_commits(commit_hash: String) -> Vec<Commit> {
    let mut queue = VecDeque::new();
    let mut commit_set: HashSet<String> = HashSet::new();
    let mut reachable_commits: Vec<Commit> = Vec::new();
    queue.push_back(commit_hash);

    while !queue.is_empty() {
        let commit_id = queue.pop_front().unwrap();
        let commit_id_hash = SHA1::from_str(&commit_id).unwrap();
        let commit = load_object::<Commit>(&commit_id_hash)
            .expect("fatal: storage broken, object not found");

        if commit_set.contains(&commit_id) {
            continue;
        }
        commit_set.insert(commit_id.clone());

        for parent_commit_id in &commit.parent_commit_ids {
            queue.push_back(parent_commit_id.to_string());
        }

        reachable_commits.push(commit);
    }

    reachable_commits
}

/// Execute the log command
pub async fn execute(args: LogArgs) {
    #[cfg(unix)]
    let mut process = Command::new("less")
        .arg("-R")
        .arg("-F")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()
        .expect("failed to execute process");

    let head = Head::current().await;

    if let Head::Branch(branch_name) = head.to_owned() {
        let branch = Branch::find_branch(&branch_name, None).await;
        if branch.is_none() {
            panic!("fatal: your current branch '{branch_name}' does not have any commits yet");
        }
    }

    let commit_hash = Head::current_commit().await.unwrap().to_string();
    let mut reachable_commits = get_reachable_commits(commit_hash.clone()).await;
    reachable_commits.sort_by(|a, b| b.committer.timestamp.cmp(&a.committer.timestamp));

    let branch_commits = create_branch_commits_map().await;
    let max_output_number = min(args.number.unwrap_or(usize::MAX), reachable_commits.len());
    let mut output_number = 0;

    for commit in reachable_commits {
        if output_number >= max_output_number {
            break;
        }
        output_number += 1;

        let branches = branch_commits.get(&commit.id).cloned().unwrap_or_default();
        let paths: Vec<PathBuf> = args.pathspec.iter().map(util::to_workdir_path).collect();

        let message = if args.oneline {
            // Short hash
            let short_hash = &commit.id.to_string()[..7];
            let (msg, _) = parse_commit_msg(&commit.message);

            let mut ref_info = Vec::new();
            if let Head::Branch(ref current_branch) = head {
                if branches.contains(current_branch) {
                    ref_info.push(format!("HEAD -> {}", current_branch.green()));
                }
            }

            let other_branches: Vec<String> = branches
                .iter()
                .filter(|b| match &head {
                    Head::Branch(name) => b != name,
                    _ => true,
                })
                .map(|b| b.green().to_string())
                .collect();
            ref_info.extend(other_branches);

            if !ref_info.is_empty() {
                format!("{} {} ({})", short_hash.yellow().bold(), msg, ref_info.join(", "))
            } else {
                format!("{} {}", short_hash.yellow().bold(), msg)
            }
        } else {
            // Detailed format
            let mut message = format!(
                "{} {}",
                "commit".yellow(),
                if !branches.is_empty() {
                    commit.id.to_string().yellow().bold()
                } else {
                    commit.id.to_string().yellow()
                }
            );

            if output_number == 1 {
                let mut refs = vec![];
                let current_branch = if let Head::Branch(name) = head.to_owned() {
                    refs.push(format!("{} -> {}", "HEAD".blue(), name.green()));
                    Some(name)
                } else {
                    refs.push("HEAD".blue().to_string());
                    None
                };

                let other_branches: Vec<String> = branches
                    .iter()
                    .filter(|&b| current_branch.as_ref() != Some(b))
                    .map(|b| b.green().to_string())
                    .collect();

                refs.extend(other_branches);
                message = format!("{} ({})", message, refs.join(", "));
            } else if !branches.is_empty() {
                message = format!("{} ({})", message, branches.join(", ").green());
            }

            message.push_str(&format!("\nAuthor: {}", commit.author));
            let (msg, _) = parse_commit_msg(&commit.message);
            message.push_str(&format!("\n{msg}\n"));

            if args.patch {
                let patch_output = generate_diff(&commit, paths.clone()).await;
                message.push_str(&patch_output);
            }

            message
        };

        #[cfg(unix)]
        {
            if let Some(ref mut stdin) = process.stdin {
                writeln!(stdin, "{message}").unwrap();
            } else {
                eprintln!("Failed to capture stdin");
            }
        }
        #[cfg(not(unix))]
        {
            println!("{message}");
        }
    }

    #[cfg(unix)]
    {
        let _ = process.wait().expect("failed to wait on child");
    }
}

/// Map commit hashes to branch names
async fn create_branch_commits_map() -> HashMap<SHA1, Vec<String>> {
    let all_branches = Branch::list_branches(None).await;
    let mut commit_to_branches: HashMap<SHA1, Vec<String>> = HashMap::new();

    for branch in all_branches {
        let branch_name = match &branch.remote {
            Some(remote) => format!("{}/{}", remote, branch.name),
            None => branch.name,
        };
        commit_to_branches.entry(branch.commit).or_default().push(branch_name);
    }

    commit_to_branches
}

/// Generate diff for a commit
async fn generate_diff(commit: &Commit, paths: Vec<PathBuf>) -> String {
    let tree = load_object::<Tree>(&commit.tree_id).unwrap();
    let new_blobs: Vec<(PathBuf, SHA1)> = tree.get_plain_items();

    let old_blobs: Vec<(PathBuf, SHA1)> = if !commit.parent_commit_ids.is_empty() {
        let parent = &commit.parent_commit_ids[0];
        let parent_hash = SHA1::from_str(&parent.to_string()).unwrap();
        let parent_commit = load_object::<Commit>(&parent_hash).unwrap();
        let parent_tree = load_object::<Tree>(&parent_commit.tree_id).unwrap();
        parent_tree.get_plain_items()
    } else {
        Vec::new()
    };

    let read_content = |file: &PathBuf, hash: &SHA1| match load_object::<Blob>(hash) {
        Ok(blob) => blob.data,
        Err(_) => {
            let file = util::to_workdir_path(file);
            std::fs::read(&file).unwrap()
        }
    };

    let diffs = Diff::diff(
        old_blobs,
        new_blobs,
        String::from("histogram"),
        paths.into_iter().collect(),
        read_content,
    )
    .await;

    let mut out = String::new();
    for d in diffs {
        out.push_str(&d.data);
    }
    out
}

#[cfg(test)]
mod tests {}
