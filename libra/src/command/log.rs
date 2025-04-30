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
            panic!(
                "fatal: your current branch '{}' does not have any commits yet ",
                branch_name
            );
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
        let mut message = {
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
            message
        };
        message.push_str(&format!("\nAuthor: {}", commit.author));
        let (msg, _) = parse_commit_msg(&commit.message);
        message.push_str(&format!("\n{}\n", msg));

        #[cfg(unix)]
        {
            if let Some(ref mut stdin) = process.stdin {
                writeln!(stdin, "{}", message).unwrap();
            } else {
                eprintln!("Failed to capture stdin");
            }
        }
        #[cfg(not(unix))]
        {
            println!("{}", message);
        }
    }
    #[cfg(unix)]
    {
        let _ = process.wait().expect("failed to wait on child");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{command::save_object, utils::test};
    use common::utils::format_commit_msg;
    use mercury::{hash::SHA1, internal::object::commit::Commit};
    use serial_test::serial;
    use tempfile::tempdir;

    #[tokio::test]
    #[serial]
    async fn test_get_reachable_commits() {
        let temp_path = tempdir().unwrap();
        test::setup_with_new_libra_in(temp_path.path()).await;
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        let commit_id = create_test_commit_tree().await;

        let reachable_commits = get_reachable_commits(commit_id).await;
        assert_eq!(reachable_commits.len(), 6);
    }

    #[tokio::test]
    #[ignore] // ignore this test because it will open less and block the test
    async fn test_execute_log() {
        let temp_path = tempdir().unwrap();
        test::setup_with_new_libra_in(temp_path.path()).await;
        let _ = create_test_commit_tree().await;

        let args = LogArgs { number: Some(6) };
        execute(args).await;
    }

    /// create a test commit tree structure as graph and create branch (master) head to commit 6
    /// return a commit hash of commit 6
    ///            3   6
    ///          /  \ /
    ///    1 -- 2    5
    //           \  / \
    ///            4   7
    async fn create_test_commit_tree() -> String {
        let mut commit_1 = Commit::from_tree_id(
            SHA1::new(&[1; 20]),
            vec![],
            &format_commit_msg("Commit_1", None),
        );
        commit_1.committer.timestamp = 1;
        // save_object(&commit_1);
        save_object(&commit_1, &commit_1.id).unwrap();

        let mut commit_2 = Commit::from_tree_id(
            SHA1::new(&[2; 20]),
            vec![commit_1.id],
            &format_commit_msg("Commit_2", None),
        );
        commit_2.committer.timestamp = 2;
        save_object(&commit_2, &commit_2.id).unwrap();

        let mut commit_3 = Commit::from_tree_id(
            SHA1::new(&[3; 20]),
            vec![commit_2.id],
            &format_commit_msg("Commit_3", None),
        );
        commit_3.committer.timestamp = 3;
        save_object(&commit_3, &commit_3.id).unwrap();

        let mut commit_4 = Commit::from_tree_id(
            SHA1::new(&[4; 20]),
            vec![commit_2.id],
            &format_commit_msg("Commit_4", None),
        );
        commit_4.committer.timestamp = 4;
        save_object(&commit_4, &commit_4.id).unwrap();

        let mut commit_5 = Commit::from_tree_id(
            SHA1::new(&[5; 20]),
            vec![commit_2.id, commit_4.id],
            &format_commit_msg("Commit_5", None),
        );
        commit_5.committer.timestamp = 5;
        save_object(&commit_5, &commit_5.id).unwrap();

        let mut commit_6 = Commit::from_tree_id(
            SHA1::new(&[6; 20]),
            vec![commit_3.id, commit_5.id],
            &format_commit_msg("Commit_6", None),
        );
        commit_6.committer.timestamp = 6;
        save_object(&commit_6, &commit_6.id).unwrap();

        let mut commit_7 = Commit::from_tree_id(
            SHA1::new(&[7; 20]),
            vec![commit_5.id],
            &format_commit_msg("Commit_7", None),
        );
        commit_7.committer.timestamp = 7;
        save_object(&commit_7, &commit_7.id).unwrap();

        // set current branch head to commit 6
        let head = Head::current().await;
        let branch_name = match head {
            Head::Branch(name) => name,
            _ => panic!("should be branch"),
        };

        Branch::update_branch(&branch_name, &commit_6.id.to_string(), None).await;

        commit_6.id.to_string()
    }
}
