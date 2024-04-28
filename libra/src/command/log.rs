use std::collections::HashSet;

use crate::command::load_object;
use crate::db;
use crate::model::reference;
use crate::utils::util;
use clap::Parser;
use colored::Colorize;
#[cfg(unix)]
use pager::Pager;
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
    {
        // init a pager to show the output
        let mut pager = Pager::new();
        pager.setup();
    }

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

    let mut commit_hash = reference::Model::current_commit_hash(&db)
        .await
        .unwrap()
        .unwrap();
    let mut output_number = 0;
    // loop {
    //     if args.number.is_some() && output_number >= args.number.unwrap() {
    //         break;
    //     }
    //     output_number += 1;

    //     let commit = load_object::<Commit>(&commit_hash, &storage)
    //         .await
    //         .expect("fatal: storage broken, object not found");
    //     let commit_message = {
    //         let mut message = format!("{} {}", "commit".yellow(), &commit_hash.yellow());

    //         if output_number == 1 {
    //             message += &format!("{}{}", &"(".yellow(), &"HEAD".yellow());
    //             if head.name.is_some() {
    //                 message += &"-> ".blue();
    //                 message += &head.name.as_ref().unwrap().green();
    //             }
    //         }
    //         message += &")".yellow().to_string();
    //         message
    //     };
    //     println!("{}", commit_message);
    //     println!("Author: {}", commit.author);
    //     println!("{}", commit.message);

    //     if commit.parent_commit_ids.is_empty() {
    //         break;
    //     }
    //     // TODO: currect order
    //     // git log use a combine of topological order and time order, and will show all
    //     commit_hash = commit.parent_commit_ids[0].to_string();
    // }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::utils::{test, util};
    use storage::driver::file_storage::FileStorage;
    use venus::{
        hash::SHA1,
        internal::object::{commit::Commit, signature::Signature, ObjectTrait},
    };

    /// create a test commit tree structure as graph and return a commit hash of commit 6
    ///            3   6
    ///          /  \ /
    ///    1 -- 2    5
    //           \  / \
    ///            4   7
    fn create_test_commit_tree() -> String {
        fn save_commit(commit: &Commit) {
            let data = commit.to_data().unwrap();
            let storage = util::objects_storage();
            storage
                .put(&commit.id, &data)
                .unwrap();
        }
        let storage = util::objects_storage();
        let commit_1 = Commit::from_tree_id(SHA1::new(&vec![1; 20]), vec![], "Commit_1");

        save_commit(&commit_1);
        let commit_2 = Commit::from_tree_id(SHA1::new(&vec![2; 20]), vec![commit_1.id], "Commit_2");
        save_commit(&commit_2);

        let commit_3 = Commit::from_tree_id(SHA1::new(&vec![3; 20]), vec![commit_2.id], "Commit_3");
        save_commit(&commit_3);

        let commit_4 = Commit::from_tree_id(SHA1::new(&vec![4; 20]), vec![commit_2.id], "Commit_4");
        save_commit(&commit_4);

        let commit_5 = Commit::from_tree_id(
            SHA1::new(&vec![5; 20]),
            vec![commit_2.id, commit_4.id],
            "Commit_5",
        );
        save_commit(&commit_5);

        let commit_6 = Commit::from_tree_id(
            SHA1::new(&vec![6; 20]),
            vec![commit_3.id, commit_5.id],
            "Commit_6",
        );
        save_commit(&commit_6);

        let commit_7 = Commit::from_tree_id(SHA1::new(&vec![7; 20]), vec![commit_5.id], "Commit_7");
        save_commit(&commit_7);

        commit_6.id.to_plain_str()
    }

    #[tokio::test]
    async fn test_get_reachable_commits() {
        test::setup_with_new_libra().await;
        let commit_id = create_test_commit_tree();

        let reachable_commits = get_reachable_commits(commit_id).await;
        assert_eq!(reachable_commits.len(), 6);
    }
}
