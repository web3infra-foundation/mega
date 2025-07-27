use colored::Colorize;
use libra::command::branch;
use libra::command::checkout::check_branch;
use libra::command::checkout::get_current_branch;
use libra::command::checkout::switch_branch;
use libra::{
    command::{commit, init},
    utils::test,
};
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
/// Tests branch creation, switching and validation functionality in the checkout module.
/// Verifies proper branch management and HEAD reference updates when switching between branches.
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
        signoff: false,
        disable_pre: true,
    };
    commit::execute(commit_args).await;

    // Create tow new branch
    branch::create_branch(String::from("new_branch_01"), get_current_branch().await).await;
    branch::create_branch(String::from("new_branch_02"), get_current_branch().await).await;

    // Test the checkout module funsctions
    test_check_branch().await;
    test_switch_branch().await;
}
