use std::collections::HashSet;

use crate::command::load_object;
use crate::db;
use crate::model::reference;
use clap::Parser;
use colored::Colorize;
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::process::{Command, Stdio};

use std::collections::VecDeque;
use std::str::FromStr;
use venus::hash::SHA1;
use venus::internal::object::commit::Commit;
#[derive(Parser, Debug)]
pub struct LogArgs {
    /// Limit the number of output
    #[clap(short, long)]
    pub number: Option<usize>,
}

///  Get all reachable commits from the given commit hash
///  **didn't consider the order of the commits**
async fn get_reachable_commits(commit_hash: String) -> Vec<Commit> {
    let mut queue = VecDeque::new();
    let mut commit_set: HashSet<String> = HashSet::new(); // to avoid duplicate commits because of circular reference
    let mut reachable_commits: Vec<Commit> = Vec::new();
    queue.push_back(commit_hash);

    while !queue.is_empty() {
        let commit_id = queue.pop_front().unwrap();

        let commit = load_object::<Commit>(&SHA1::from_str(&commit_id).unwrap())
            .expect("fatal: storage broken, object not found");
        if commit_set.contains(&commit_id) {
            continue;
        }
        commit_set.insert(commit_id);

        let parent_commit_ids = commit.parent_commit_ids.clone();
        for parent_commit_id in parent_commit_ids {
            queue.push_back(parent_commit_id.to_plain_str());
        }
        reachable_commits.push(commit);
    }
    reachable_commits
}

pub async fn execute(args: LogArgs) {
    #[cfg(unix)]
    let mut process = Command::new("less") // create a pipe to less
        .arg("-R") // raw control characters
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()
        .expect("failed to execute process");

    let db = db::get_db_conn().await.unwrap();
    let head = reference::Model::current_head(&db).await.unwrap();

    // check if the current branch has any commits
    if head.name.is_some() {
        let branch_name = head.name.as_ref().unwrap();
        let branch = reference::Model::find_branch_by_name(&db, branch_name)
            .await
            .unwrap();
        if branch.is_none() {
            panic!(
                "fatal: your current branch '{}' does not have any commits yet ",
                branch_name
            );
        }
    }

    let commit_hash = reference::Model::current_commit_hash(&db)
        .await
        .unwrap()
        .unwrap();
    let mut reachable_commits = get_reachable_commits(commit_hash.clone()).await;
    // default sort with signature time
    reachable_commits.sort_by(|a, b| a.committer.timestamp.cmp(&b.committer.timestamp));

    let mut output_number = 0;
    for commit in reachable_commits {
        if args.number.is_some() && output_number >= args.number.unwrap() {
            break;
        }
        output_number += 1;
        let mut message = {
            let mut message = format!(
                "{} {}",
                "commit".yellow(),
                &commit.id.to_plain_str().yellow()
            );

            // TODO other branch's head should shown branch name
            if output_number == 1 {
                message = format!("{} {}{}", message, "(".yellow(), "HEAD".blue());
                if head.name.is_some() {
                    // message += &"-> ".blue();
                    // message += &head.name.as_ref().unwrap().green();
                    message = format!(
                        "{}{}{}",
                        message,
                        " -> ".blue(),
                        head.name.as_ref().unwrap().green()
                    );
                }
            }
            message = format!("{}{}", message, ")".yellow());
            message
        };
        message.push_str(&format!("\nAuthor: {}", commit.author));
        message.push_str(&format!("\n{}\n", commit.message));

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
    use crate::utils::{test, util};
    use sea_orm::{ActiveModelTrait, Set};
    use tests::reference::ActiveModel;
    use venus::{
        hash::SHA1,
        internal::object::{commit::Commit, ObjectTrait},
    };

    /// create a test commit tree structure as graph and create branch (master) head to commit 6
    /// return a commit hash of commit 6
    ///            3   6
    ///          /  \ /
    ///    1 -- 2    5
    //           \  / \
    ///            4   7
    async fn create_test_commit_tree() -> String {
        fn save_commit(commit: &Commit) {
            let data = commit.to_data().unwrap();
            let storage = util::objects_storage();
            storage.put(&commit.id, &data, commit.get_type()).unwrap();
        }
        let mut commit_1 = Commit::from_tree_id(SHA1::new(&vec![1; 20]), vec![], "Commit_1");
        commit_1.committer.timestamp = 1;
        save_commit(&commit_1);

        let mut commit_2 =
            Commit::from_tree_id(SHA1::new(&vec![2; 20]), vec![commit_1.id], "Commit_2");
        commit_2.committer.timestamp = 2;
        save_commit(&commit_2);

        let mut commit_3 =
            Commit::from_tree_id(SHA1::new(&vec![3; 20]), vec![commit_2.id], "Commit_3");
        commit_3.committer.timestamp = 3;
        save_commit(&commit_3);

        let mut commit_4 =
            Commit::from_tree_id(SHA1::new(&vec![4; 20]), vec![commit_2.id], "Commit_4");
        commit_4.committer.timestamp = 4;
        save_commit(&commit_4);

        let mut commit_5 = Commit::from_tree_id(
            SHA1::new(&vec![5; 20]),
            vec![commit_2.id, commit_4.id],
            "Commit_5",
        );
        commit_5.committer.timestamp = 5;
        save_commit(&commit_5);

        let mut commit_6 = Commit::from_tree_id(
            SHA1::new(&vec![6; 20]),
            vec![commit_3.id, commit_5.id],
            "Commit_6",
        );
        commit_6.committer.timestamp = 6;
        save_commit(&commit_6);

        let mut commit_7 =
            Commit::from_tree_id(SHA1::new(&vec![7; 20]), vec![commit_5.id], "Commit_7");
        commit_7.committer.timestamp = 7;
        save_commit(&commit_7);

        // set current branch head to commit 6
        let db = db::get_db_conn().await.unwrap();
        let head = reference::Model::current_head(&db).await.unwrap();
        let branch_name = head.name.unwrap();
        // set current branch head to commit 6
        let branch = ActiveModel {
            name: Set(Some(branch_name.clone())),
            commit: Set(Some(commit_6.id.to_plain_str())),
            kind: Set(reference::ConfigKind::Branch),
            ..Default::default()
        };
        branch.save(&db).await.unwrap();

        commit_6.id.to_plain_str()
    }

    #[tokio::test]
    async fn test_get_reachable_commits() {
        test::setup_with_new_libra().await;
        let commit_id = create_test_commit_tree().await;

        let reachable_commits = get_reachable_commits(commit_id).await;
        assert_eq!(reachable_commits.len(), 6);
    }

    #[tokio::test]
    async fn test_execute_log() {
        test::setup_with_new_libra().await;
        let _ = create_test_commit_tree().await;

        let args = LogArgs { number: None };
        execute(args).await;
    }
}
