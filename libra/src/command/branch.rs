use crate::utils::util;
use clap::Parser;
use colored::Colorize;
use sea_orm::{ActiveModelTrait, Set};
use std::str::FromStr;
use venus::hash::SHA1;
use venus::internal::object::commit::Commit;

use crate::{
    command::load_object,
    db,
    model::reference::{self, ConfigKind},
};

#[derive(Parser, Debug)]
pub struct BranchArgs {
    /// new branch name
    #[clap(group = "sub")]
    new_branch: Option<String>,

    /// base branch name or commit hash
    #[clap(requires = "new_branch")]
    commit_hash: Option<String>,

    /// list all branches
    #[clap(short, long, action, group = "sub", default_value = "true")]
    list: bool,

    /// force delete branch
    #[clap(short = 'D', long, group = "sub")]
    delete: Option<String>,

    /// show current branch
    #[clap(long, action, group = "sub")]
    show_curren: bool,
}
pub async fn execute(args: BranchArgs) {
    if args.new_branch.is_some() {
        create_branch(args.new_branch.unwrap(), args.commit_hash).await;
    } else if args.delete.is_some() {
        delete_branch(args.delete.unwrap()).await;
    } else if args.show_curren {
        show_current_branch().await;
    } else if args.list {
        // 兜底list
        list_branches().await;
    } else {
        panic!("should not reach here")
    }
}

pub async fn create_branch(new_branch: String, branch_or_commit: Option<String>) {
    // commit hash maybe a branch name
    let db = db::get_db_conn().await.unwrap();
    let commit_hash = match branch_or_commit {
        Some(branch_or_commit) => {
            let branch = reference::Model::find_branch_by_name(&db, &branch_or_commit)
                .await
                .unwrap();
            match branch {
                Some(branch) => branch.commit.unwrap(),
                None => {
                    let commit_base = util::get_commit_base(&branch_or_commit).unwrap();
                    commit_base.to_plain_str()
                }
            }
        }
        None => {
            let head = reference::Model::current_head(&db).await.unwrap();
            match head.commit {
                Some(commit) => commit,
                None => {
                    let current_branch_name = head.name.unwrap();
                    let branch = reference::Model::find_branch_by_name(&db, &current_branch_name)
                        .await
                        .unwrap()
                        .unwrap_or_else(|| {
                            panic!("fatal: no branch named '{}'", current_branch_name)
                        });
                    branch.commit.unwrap()
                }
            }
        }
    };
    // check if commit_hash exists
    let _ = load_object::<Commit>(&SHA1::from_str(&commit_hash).unwrap())
        .unwrap_or_else(|_| panic!("fatal: not a valid object name: '{}'", commit_hash));

    // create branch
    let branch = reference::ActiveModel {
        name: Set(Some(new_branch)),
        kind: Set(ConfigKind::Branch),
        commit: Set(Some(commit_hash)),
        ..Default::default()
    };
    branch.save(&db).await.unwrap();
}

async fn delete_branch(branch_name: String) {
    let db = db::get_db_conn().await.unwrap();
    let branch = reference::Model::find_branch_by_name(&db, &branch_name)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("fatal: branch '{}' not found", branch_name));
    let head = reference::Model::current_head(&db).await.unwrap();

    // can't delete current branch
    if head.name.is_some() && head.name.unwrap() == branch_name {
        panic!(
            "fatal: Cannot delete the branch '{}' which you are currently on",
            branch_name
        );
    }

    let branch: reference::ActiveModel = branch.into();
    branch.delete(&db).await.unwrap();
}

async fn show_current_branch() {
    let db = db::get_db_conn().await.unwrap();
    let head = reference::Model::current_head(&db).await.unwrap();
    if head.name.is_none() {
        println!("HEAD detached at {}", &head.commit.unwrap()[..8]);
    } else {
        println!("{}", head.name.unwrap());
    }
}

async fn list_branches() {
    let db = db::get_db_conn().await.unwrap();
    let branches = reference::Model::find_all_branches(&db, None)
        .await
        .unwrap();
    let head = reference::Model::current_head(&db).await.unwrap();
    let is_detached = head.name.is_none();
    if is_detached {
        let s = "HEAD detached at  ".to_string() + &head.commit.unwrap()[..8];
        let s = s.green();
        println!("{}", s);
    };
    let head_name = head.name.unwrap_or_default();
    for branch in branches {
        let name = branch.name.unwrap();
        if head_name == name {
            println!("* {}", name.green());
        } else {
            println!("  {}", name);
        };
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        command::commit::{self, CommitArgs},
        utils::test,
    };

    use super::*;

    #[tokio::test]
    async fn test_branch() {
        test::setup_with_new_libra().await;
        let db = db::get_db_conn().await.unwrap();

        let commit_args = CommitArgs {
            message: "first".to_string(),
            allow_empty: true,
        };
        commit::execute(commit_args).await;
        let first_commit_id = reference::Model::find_branch_by_name(&db, "master")
            .await
            .unwrap()
            .unwrap()
            .commit
            .unwrap();

        let commit_args = CommitArgs {
            message: "second".to_string(),
            allow_empty: true,
        };
        commit::execute(commit_args).await;
        let second_commit_id = reference::Model::find_branch_by_name(&db, "master")
            .await
            .unwrap()
            .unwrap()
            .commit
            .unwrap();

        {
            // create branch with first commit
            let first_branch_name = "first_branch".to_string();
            let args = BranchArgs {
                new_branch: Some(first_branch_name.clone()),
                commit_hash: Some(first_commit_id.clone()),
                list: false,
                delete: None,
                show_curren: false,
            };
            execute(args).await;

            // check branch exist
            let current_branch = reference::Model::current_head(&db)
                .await
                .unwrap()
                .name
                .unwrap();
            assert_ne!(current_branch, first_branch_name);
            let first_branch = reference::Model::find_branch_by_name(&db, &first_branch_name)
                .await
                .unwrap()
                .unwrap();
            assert!(first_branch.commit.unwrap() == first_commit_id);
            assert!(first_branch.name.unwrap() == first_branch_name);
        }

        {
            // create second branch with current branch
            let second_branch_name = "second_branch".to_string();
            let args = BranchArgs {
                new_branch: Some(second_branch_name.clone()),
                commit_hash: None,
                list: false,
                delete: None,
                show_curren: false,
            };
            execute(args).await;
            let second_branch = reference::Model::find_branch_by_name(&db, &second_branch_name)
                .await
                .unwrap()
                .unwrap();
            assert!(second_branch.commit.unwrap() == second_commit_id);
            assert!(second_branch.name.unwrap() == second_branch_name);
        }

        // show current branch
        println!("show current branch");
        let args = BranchArgs {
            new_branch: None,
            commit_hash: None,
            list: false,
            delete: None,
            show_curren: true,
        };
        execute(args).await;

        // list branches
        println!("list branches");
        execute(BranchArgs::parse_from([""])).await; // default list
    }
}
