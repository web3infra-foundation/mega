use clap::Parser;
use mercury::hash::SHA1;

use crate::{
    command::branch,
    internal::{branch::Branch, head::Head},
    utils::util::{self, get_commit_base},
};

use super::{
    restore::{self, RestoreArgs},
    status,
};

#[derive(Parser, Debug)]
pub struct SwitchArgs {
    /// branch name
    #[clap(required_unless_present("create"), required_unless_present("detach"))]
    branch: Option<String>,

    /// Create a new branch based on the given branch or current HEAD, and switch to it
    #[clap(long, short, group = "sub")]
    create: Option<String>,

    /// Switch to a commit
    #[clap(long, short, action, default_value = "false", group = "sub")]
    detach: bool,
}

pub async fn execute(args: SwitchArgs) {
    // check status
    let unstaged = status::changes_to_be_staged();
    if !unstaged.deleted.is_empty() || !unstaged.modified.is_empty() {
        status::execute().await;
        eprintln!("fatal: uncommitted changes, can't switch branch");
        return;
    } else if !status::changes_to_be_committed().await.is_empty() {
        status::execute().await;
        eprintln!("fatal: unstaged changes, can't switch branch");
        return;
    }

    match args.create {
        Some(new_branch_name) => {
            branch::create_branch(new_branch_name.clone(), args.branch).await;
            switch_to_branch(new_branch_name).await;
        }
        None => match args.detach {
            true => {
                let commit_base = get_commit_base(&args.branch.unwrap());
                if commit_base.is_err() {
                    eprintln!("{}", commit_base.unwrap());
                    return;
                }
                switch_to_commit(commit_base.unwrap()).await;
            }
            false => {
                switch_to_branch(args.branch.unwrap()).await;
            }
        },
    }
}

// Check status before change the branch
pub async fn check_status() -> bool {
    let unstaged: status::Changes = status::changes_to_be_staged();
    if !unstaged.deleted.is_empty() || !unstaged.modified.is_empty() {
        status::execute().await;
        eprintln!("fatal: uncommitted changes, can't switch branch");
        true
    } else if !status::changes_to_be_committed().await.is_empty() {
        status::execute().await;
        eprintln!("fatal: unstaged changes, can't switch branch");
        true
    } else {
        false
    }
}

/// change the working directory to the version of commit_hash
async fn switch_to_commit(commit_hash: SHA1) {
    restore_to_commit(commit_hash).await;
    // update HEAD
    let head = Head::Detached(commit_hash);
    Head::update(head, None).await;
}

async fn switch_to_branch(branch_name: String) {
    let target_branch = Branch::find_branch(&branch_name, None).await;
    if target_branch.is_none() {
        if !Branch::search_branch(&branch_name).await.is_empty() {
            eprintln!(
                "fatal: a branch is expected, got remote branch {}",
                branch_name
            );
        } else {
            eprintln!("fatal: branch '{}' not found", &branch_name);
        }
        return;
    }
    let commit_id = target_branch.unwrap().commit;
    restore_to_commit(commit_id).await;
    // update HEAD
    // let mut head: ActiveModel = reference::Model::current_head(db).await.unwrap().into();
    let head = Head::Branch(branch_name);
    Head::update(head, None).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::add;
    use crate::command::init;
    use crate::command::restore::RestoreArgs;
    use serial_test::serial;
    use std::str::FromStr;
    use std::{env, fs};
    use tempfile::tempdir;
    #[test]
    fn test_parse_from() {
        let commit_id = SHA1::from_str("0cb5eb6281e1c0df48a70716869686c694706189").unwrap();
        let restore_args = RestoreArgs::parse_from([
            "restore", // important, the first will be ignored
            "--worktree",
            "--staged",
            "--source",
            &commit_id.to_string(),
            "./",
        ]);
        println!("{:?}", restore_args);
    }

    async fn test_check_status() {
        println!("\n\x1b[1mTest check_status function.\x1b[0m");

        // Test the check_status
        // Expect false when no changes
        assert!(!check_status().await);

        // Create a file and add it to the index
        // Expect true when there are unstaged changes
        fs::File::create("foo.txt").unwrap();
        let add_args = add::AddArgs {
            pathspec: vec!["foo.txt".to_string()],
            all: false,
            update: false,
            verbose: true,
        };
        add::execute(add_args).await;
        assert!(check_status().await);

        // Modify a file
        // Expect true when there are uncommitted changes
        fs::write("foo.txt", "modified content").unwrap();
        assert!(check_status().await);
    }

    #[tokio::test]
    #[serial]
    async fn test_parts_of_switch_module_function() {
        println!("\n\x1b[1mTest some functions of the switch module.\x1b[0m");

        let target_dir = tempdir().unwrap().into_path();

        // Create a test directory and set args
        let test_dir = target_dir.join("test_check_status");
        fs::create_dir(&test_dir).unwrap();

        let init_args = init::InitArgs {
            bare: false,
            initial_branch: None,
            repo_directory: test_dir.to_str().unwrap().to_owned(),
            quiet: false,
        };

        // Run the init function and change the current directory to the test directory
        let raw_dir = env::current_dir().unwrap();
        let result = init::init(init_args).await;
        if let Err(e) = result {
            eprintln!("Error initializing repository: {}", e);
            return;
        }
        assert!(env::set_current_dir(&test_dir).is_ok());

        // Test the switch module funsctions
        test_check_status().await;

        // Clean the test data
        assert!(env::set_current_dir(&raw_dir).is_ok());
        if let Err(e) = fs::remove_dir_all(&target_dir) {
            eprintln!("Error removing test directory: {}", e);
        }
    }
}
