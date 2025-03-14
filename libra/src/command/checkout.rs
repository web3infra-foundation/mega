use clap::Parser;
use mercury::hash::SHA1;

use crate::{
    internal::{branch::Branch, config::Config, head::Head},
    utils::util,
};

use super::{
    branch, fetch, merge,
    restore::{self, RestoreArgs},
    switch,
};

#[derive(Parser, Debug)]
pub struct CheckoutArgs {
    /// Target branche name
    branch: Option<String>,

    /// Create and switch to a new branch with the same content as the current branch
    #[clap(short = 'b', group = "sub")]
    new_branch: Option<String>,
}

pub async fn execute(args: CheckoutArgs) {
    if switch::check_status().await {
        return;
    }

    match (args.branch, args.new_branch) {
        (Some(target_branch), _) => check_and_switch_branch(&target_branch).await,
        (None, Some(new_branch)) => create_and_switch_new_branch(&new_branch).await,
        (None, None) => show_current_branch().await,
    }
}

async fn get_current_branch() -> Option<String> {
    let head = Head::current().await;
    match head {
        Head::Detached(commit_hash) => {
            println!("HEAD detached at {}", &commit_hash.to_string()[..8]);
            None
        }
        Head::Branch(name) => Some(name),
    }
}

async fn show_current_branch() {
    if let Some(current_branch) = get_current_branch().await {
        println!("Current branch is {current_branch}.");
    }
}

pub async fn pull_upstream() {
    fetch::execute(fetch::FetchArgs {
        repository: None,
        refspec: None,
        all: false,
    })
    .await;

    let head = Head::current().await;
    match head {
        Head::Branch(name) => match Config::branch_config(&name).await {
            Some(branch_config) => {
                let merge_args = merge::MergeArgs {
                    branch: format!("{}/{}", branch_config.remote, branch_config.merge),
                };
                merge::execute(merge_args).await;
            }
            None => {
                eprintln!("There is no tracking information for the current branch.");
                eprintln!("hint: set up a tracking branch with `libra branch --set-upstream-to=<remote>/<branch>`")
            }
        },
        _ => {
            eprintln!("You are not currently on a branch.");
        }
    }
}

async fn switch_branch(branch_name: &str) {
    let target_branch: Option<Branch> = Branch::find_branch(branch_name, None).await;
    let commit_id = target_branch.unwrap().commit;
    restore_to_commit(commit_id).await;

    let head = Head::Branch(branch_name.to_string());
    Head::update(head, None).await;
}

async fn create_and_switch_new_branch(new_branch: &str) {
    branch::create_branch(new_branch.to_string(), get_current_branch().await).await;
    switch_branch(new_branch).await;
    println!("Switched to a new branch '{new_branch}'");
}

async fn check_branch_and_get_remote(branch_name: &str) -> bool {
    if get_current_branch().await == Some(branch_name.to_string()) {
        println!("Already on {branch_name}");
        return true;
    }

    let target_branch: Option<Branch> = Branch::find_branch(branch_name, None).await;
    if target_branch.is_none() {
        let remote_branch_name: String = format!("origin/{}", branch_name);
        if !Branch::search_branch(&remote_branch_name).await.is_empty() {
            println!("branch '{branch_name}' set up to track '{remote_branch_name}'.");

            create_and_switch_new_branch(branch_name).await;
            // Set branch upstream
            branch::set_upstream(branch_name, &remote_branch_name).await;
            // Synchronous branches
            pull_upstream().await;

            false
        } else {
            eprintln!("fatal: branch '{}' not found", &branch_name);
            true
        }
    } else {
        println!("Switched to branch '{branch_name}'");
        false
    }
}

async fn check_and_switch_branch(branch_name: &str) {
    if check_branch_and_get_remote(branch_name).await {
        return;
    }
    switch_branch(branch_name).await;
}

async fn restore_to_commit(commit_id: SHA1) {
    let restore_args = RestoreArgs {
        worktree: true,
        staged: true,
        source: Some(commit_id.to_string()),
        pathspec: vec![util::working_dir_string()],
    };
    restore::execute(restore_args).await;
}
