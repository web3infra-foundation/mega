use crate::{
    internal::{branch::Branch, config::Config, head::Head},
    utils::util,
};
use clap::Parser;
use colored::Colorize;
use venus::internal::object::commit::Commit;

use crate::command::load_object;

#[derive(Parser, Debug)]
pub struct BranchArgs {
    /// new branch name
    #[clap(group = "sub")]
    new_branch: Option<String>,

    /// base branch name or commit hash
    #[clap(requires = "new_branch")]
    commit_hash: Option<String>,

    /// list all branches
    #[clap(short, long, group = "sub", default_value = "true")]
    list: bool,

    /// force delete branch
    #[clap(short = 'D', long, group = "sub")]
    delete: Option<String>,

    /// show current branch
    #[clap(long, group = "sub")]
    show_curren: bool,

    /// show remote branches
    #[clap(short, long, requires = "list")]
    remotes: bool,
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
        list_branches(args.remotes).await;
    } else {
        panic!("should not reach here")
    }
}

pub async fn create_branch(new_branch: String, branch_or_commit: Option<String>) {
    // check if branch exists
    let branch = Branch::find_branch(&new_branch, None).await;
    if branch.is_some() {
        panic!("fatal: A branch named '{}' already exists.", new_branch);
    }

    // commit hash maybe a branch name
    let commit_id = match branch_or_commit {
        Some(branch_or_commit) => {
            let branch = Branch::find_branch(&branch_or_commit, None).await;
            match branch {
                Some(branch) => branch.commit,
                None => util::get_commit_base(&branch_or_commit).unwrap(),
            }
        }
        None => Head::current_commit().await.unwrap(),
    };
    // check if commit_hash exists
    let _ = load_object::<Commit>(&commit_id)
        .unwrap_or_else(|_| panic!("fatal: not a valid object name: '{}'", commit_id));

    // create branch
    Branch::update_branch(&new_branch, &commit_id.to_plain_str(), None).await;
}

async fn delete_branch(branch_name: String) {
    let _ = Branch::find_branch(&branch_name, None)
        .await
        .unwrap_or_else(|| panic!("fatal: branch '{}' not found", branch_name));
    let head = Head::current().await;

    if let Head::Branch(name) = head {
        if name == branch_name {
            panic!(
                "fatal: Cannot delete the branch '{}' which you are currently on",
                branch_name
            );
        }
    }

    Branch::delete_branch(&branch_name, None).await;
}

async fn show_current_branch() {
    // let head = reference::Model::current_head(&db).await.unwrap();
    let head = Head::current().await;
    match head {
        Head::Detached(commit_hash) => {
            println!("HEAD detached at {}", &commit_hash.to_plain_str()[..8]);
        }
        Head::Branch(name) => {
            println!("{}", name);
        }
    }
}

async fn list_branches(remotes: bool) {
    // TODO didn't test remote branch
    let branches = match remotes {
        true => {
            // list all remote branches
            let remote_configs = Config::remote_configs().await;
            let mut branches = vec![];
            for remote in remote_configs {
                let remote_branches = Branch::lsit_branches(Some(&remote.name)).await;
                branches.extend(remote_branches);
            }
            branches
        }
        false => Branch::lsit_branches(None).await,
    };

    let head = Head::current().await;
    if let Head::Detached(commit) = head {
        let s = "HEAD detached at  ".to_string() + &commit.to_plain_str()[..8];
        let s = s.green();
        println!("{}", s);
    };
    let head_name = match head {
        Head::Branch(name) => name,
        Head::Detached(_) => "".to_string(),
    };
    for branch in branches {
        let name = branch
            .remote
            .map(|remote| remote + "/" + &branch.name)
            .unwrap_or_else(|| branch.name.clone());

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

        let commit_args = CommitArgs {
            message: "first".to_string(),
            allow_empty: true,
        };
        commit::execute(commit_args).await;
        let first_commit_id = Branch::find_branch("master", None).await.unwrap().commit;

        let commit_args = CommitArgs {
            message: "second".to_string(),
            allow_empty: true,
        };
        commit::execute(commit_args).await;
        let second_commit_id = Branch::find_branch("master", None).await.unwrap().commit;

        {
            // create branch with first commit
            let first_branch_name = "first_branch".to_string();
            let args = BranchArgs {
                new_branch: Some(first_branch_name.clone()),
                commit_hash: Some(first_commit_id.to_plain_str()),
                list: false,
                delete: None,
                show_curren: false,
                remotes: false,
            };
            execute(args).await;

            // check branch exist
            match Head::current().await {
                Head::Branch(current_branch) => {
                    assert_ne!(current_branch, first_branch_name)
                }
                _ => panic!("should be branch"),
            };

            let first_branch = Branch::find_branch(&first_branch_name, None).await.unwrap();
            assert!(first_branch.commit == first_commit_id);
            assert!(first_branch.name == first_branch_name);
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
                remotes: false,
            };
            execute(args).await;
            let second_branch = Branch::find_branch(&second_branch_name, None)
                .await
                .unwrap();
            assert!(second_branch.commit == second_commit_id);
            assert!(second_branch.name == second_branch_name);
        }

        // show current branch
        println!("show current branch");
        let args = BranchArgs {
            new_branch: None,
            commit_hash: None,
            list: false,
            delete: None,
            show_curren: true,
            remotes: false,
        };
        execute(args).await;

        // list branches
        println!("list branches");
        execute(BranchArgs::parse_from([""])).await; // default list
    }
}
