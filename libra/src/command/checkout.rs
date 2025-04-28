use clap::Parser;
use mercury::hash::SHA1;

use crate::{
    command::restore::{self, RestoreArgs},
    command::{branch, pull, switch},
    internal::{branch::Branch, head::Head},
    utils::util,
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

async fn get_remote(branch_name: &str) {
    let remote_branch_name: String = format!("origin/{}", branch_name);

    create_and_switch_new_branch(branch_name).await;
    // Set branch upstream
    branch::set_upstream(branch_name, &remote_branch_name).await;
    // Synchronous branches
    // Use the pull command to update the local branch with the latest changes from the remote branch
    pull::execute(pull::PullArgs::make(None, None)).await;
}

async fn check_branch(branch_name: &str) -> Option<bool> {
    if get_current_branch().await == Some(branch_name.to_string()) {
        println!("Already on {branch_name}");
        return None;
    }

    let target_branch: Option<Branch> = Branch::find_branch(branch_name, None).await;
    if target_branch.is_none() {
        let remote_branch_name: String = format!("origin/{}", branch_name);
        if !Branch::search_branch(&remote_branch_name).await.is_empty() {
            println!("branch '{branch_name}' set up to track '{remote_branch_name}'.");

            Some(true)
        } else {
            eprintln!(
                "fatal: Path specification '{}' did not match any files known to libra",
                &branch_name
            );
            None
        }
    } else {
        println!("Switched to branch '{branch_name}'");
        Some(false)
    }
}

async fn check_and_switch_branch(branch_name: &str) {
    match check_branch(branch_name).await {
        Some(true) => get_remote(branch_name).await,
        Some(false) => switch_branch(branch_name).await,
        None => (),
    }
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

/// Unit tests for the checkout module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        command::{commit, init},
        utils::test,
    };
    use colored::Colorize;
    use serial_test::serial;

    use tempfile::tempdir;

    async fn test_check_branch() {
        println!("\n\x1b[1mTest check_branch function.\x1b[0m");

        // For non-existent branches, it should return None
        assert_eq!(check_branch("non_existent_branch").await, None);
        // For the current branch, it should return None
        assert_eq!(
            check_branch(&get_current_branch().await.unwrap_or("main".to_string())).await,
            None
        );
        // For other existing branches, it should return Some(false)
        assert_eq!(check_branch("new_branch_01").await, Some(false));
    }

    async fn test_switch_branch() {
        println!("\n\x1b[1mTest switch_branch function.\x1b[0m");

        let show_all_branches = async || {
            // Use the list_branches function of the branch module to list all current local branches
            branch::list_branches(false).await;
            println!(
                "Current branch is '{}'.",
                get_current_branch()
                    .await
                    .unwrap_or("Get_current_branch_failed".to_string())
                    .green()
            );
        };

        // Switch to the new branch and back
        show_all_branches().await;
        switch_branch("new_branch_01").await;
        show_all_branches().await;
        switch_branch("new_branch_02").await;
        show_all_branches().await;
        switch_branch("main").await;
        show_all_branches().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_checkout_module_functions() {
        println!("\n\x1b[1mTest checkout module functions.\x1b[0m");

        let temp_path = tempdir().unwrap();
        test::setup_clean_testing_env_in(temp_path.path());
        let _guard = test::ChangeDirGuard::new(temp_path.path());

        let init_args = init::InitArgs {
            bare: false,
            initial_branch: Some("main".to_string()),
            repo_directory: temp_path.path().to_str().unwrap().to_string(),
            quiet: false,
        };

        init::init(init_args)
            .await
            .expect("Error initializing repository");

        // Initialize the main branch by creating an empty commit
        let commit_args = commit::CommitArgs {
            message: "An empty initial commit".to_string(),
            allow_empty: true,
            conventional: false,
            amend: false,
        };
        commit::execute(commit_args).await;

        // Create tow new branch
        branch::create_branch(String::from("new_branch_01"), get_current_branch().await).await;
        branch::create_branch(String::from("new_branch_02"), get_current_branch().await).await;

        // Test the checkout module funsctions
        test_check_branch().await;
        test_switch_branch().await;
    }
}
