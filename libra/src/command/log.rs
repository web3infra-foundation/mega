use std::cmp::min;
use std::collections::{HashMap, HashSet};

use crate::command::load_object;
use crate::internal::branch::Branch;
use crate::internal::head::Head;
use clap::Parser;
use colored::Colorize;
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::process::{Command, Stdio};

use mercury::hash::SHA1;
use mercury::internal::object::commit::Commit;
use std::collections::VecDeque;
use std::str::FromStr;

use common::utils::parse_commit_msg;
#[derive(Parser, Debug)]
pub struct LogArgs {
    /// Limit the number of output
    #[clap(short, long)]
    pub number: Option<usize>,
    /// Shorthand for --pretty=oneline --abbrev-commit
    #[clap(long)]
    pub oneline: bool,
}

///  Get all reachable commits from the given commit hash
///  **didn't consider the order of the commits**
pub async fn get_reachable_commits(commit_hash: String) -> Vec<Commit> {
    let mut queue = VecDeque::new();
    let mut commit_set: HashSet<String> = HashSet::new(); // to avoid duplicate commits because of circular reference
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
        commit_set.insert(commit_id);

        let parent_commit_ids = commit.parent_commit_ids.clone();
        for parent_commit_id in parent_commit_ids {
            queue.push_back(parent_commit_id.to_string());
        }
        reachable_commits.push(commit);
    }
    reachable_commits
}

pub async fn execute(args: LogArgs) {
    #[cfg(unix)]
    let mut process = Command::new("less") // create a pipe to less
        .arg("-R") // raw control characters
        .arg("-F")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()
        .expect("failed to execute process");

    let head = Head::current().await;
    // check if the current branch has any commits
    if let Head::Branch(branch_name) = head.to_owned() {
        let branch = Branch::find_branch(&branch_name, None).await;
        if branch.is_none() {
            panic!("fatal: your current branch '{branch_name}' does not have any commits yet ");
        }
    }

    let commit_hash = Head::current_commit().await.unwrap().to_string();

    let mut reachable_commits = get_reachable_commits(commit_hash.clone()).await;
    // default sort with signature time
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

        let message = if args.oneline {
            // Oneline format: <short_hash> <commit_message_first_line>
            let short_hash = &commit.id.to_string()[..7];
            let (msg, _) = parse_commit_msg(&commit.message);
            if !branches.is_empty() {
                let branch_info = format!(" ({})", branches.join(", "));
                format!(
                    "{} {}{}",
                    short_hash.yellow().bold(),
                    msg,
                    branch_info.green()
                )
            } else {
                format!("{} {}", short_hash.yellow(), msg)
            }
        } else {
            // Default detailed format
            let mut message = format!(
                "{} {}",
                "commit".yellow(),
                if !branches.is_empty() {
                    commit.id.to_string().yellow().bold()
                } else {
                    commit.id.to_string().yellow()
                }
            );

            // Show HEAD and branch info
            if output_number == 1 {
                // For the first commit (HEAD), show HEAD info and all branches
                let mut refs = vec![];
                let current_branch = if let Head::Branch(name) = head.to_owned() {
                    refs.push(format!("{} -> {}", "HEAD".blue(), name.green()));
                    Some(name)
                } else {
                    refs.push("HEAD".blue().to_string());
                    None
                };

                // Add other branches pointing to this commit (excluding current branch)
                let other_branches: Vec<String> = branches
                    .iter()
                    .filter(|&b| current_branch.as_ref() != Some(b))
                    .map(|b| b.green().to_string())
                    .collect();

                refs.extend(other_branches);

                let ref_info = format!(" ({})", refs.join(", "));
                message = format!("{message}{ref_info}");
            } else if !branches.is_empty() {
                // Show branch info for other commits that are branch heads
                let branch_info = format!(" ({})", branches.join(", "));
                message = format!("{}{}", message, branch_info.green());
            }

            message.push_str(&format!("\nAuthor: {}", commit.author));
            let (msg, _) = parse_commit_msg(&commit.message);
            message.push_str(&format!("\n{msg}\n"));
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

/// Create a map of commit hashes to branch names
async fn create_branch_commits_map() -> HashMap<SHA1, Vec<String>> {
    let all_branches = Branch::list_branches(None).await;
    let mut commit_to_branches: HashMap<SHA1, Vec<String>> = HashMap::new();

    for branch in all_branches {
        let branch_name = match &branch.remote {
            Some(remote) => format!("{}/{}", remote, branch.name),
            None => branch.name,
        };

        commit_to_branches
            .entry(branch.commit)
            .or_default()
            .push(branch_name);
    }

    commit_to_branches
}

#[cfg(test)]
mod tests {}
