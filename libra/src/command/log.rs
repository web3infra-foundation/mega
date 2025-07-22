use std::cmp::min;
use std::collections::HashSet;

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

    let max_output_number = min(args.number.unwrap_or(usize::MAX), reachable_commits.len());
    let mut output_number = 0;
    for commit in reachable_commits {
        if output_number >= max_output_number {
            break;
        }
        output_number += 1;
        let message = if args.oneline {
            // Oneline format: <short_hash> <commit_message_first_line>
            let short_hash = &commit.id.to_string()[..7];
            let (msg, _) = parse_commit_msg(&commit.message);
            format!("{} {}", short_hash.yellow(), msg)
        } else {
            // Default detailed format
            let mut message = format!("{} {}", "commit".yellow(), &commit.id.to_string().yellow());

            // TODO other branch's head should shown branch name
            if output_number == 1 {
                message = format!("{} {}{}", message, "(".yellow(), "HEAD".blue());
                if let Head::Branch(name) = head.to_owned() {
                    // message += &"-> ".blue();
                    // message += &head.name.as_ref().unwrap().green();
                    message = format!("{}{}{}", message, " -> ".blue(), name.green());
                }
                message = format!("{}{}", message, ")".yellow());
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

#[cfg(test)]
mod tests {}
