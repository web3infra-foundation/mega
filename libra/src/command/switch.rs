use clap::Parser;
use mercury::hash::SHA1;

use super::{
    restore::{self, RestoreArgs},
    status,
};
use crate::command::status::StatusArgs;
use crate::internal::db::get_db_conn_instance;
use crate::internal::reflog::{ReflogAction, ReflogContext, with_reflog};
use crate::{
    command::branch,
    internal::{branch::Branch, head::Head},
    utils::util::{self, get_commit_base},
};

#[derive(Parser, Debug)]
pub struct SwitchArgs {
    /// branch name
    #[clap(required_unless_present("create"), required_unless_present("detach"))]
    pub branch: Option<String>,

    /// Create a new branch based on the given branch or current HEAD, and switch to it
    #[clap(long, short, group = "sub")]
    pub create: Option<String>,

    /// Switch to a commit
    #[clap(long, short, action, default_value = "false", group = "sub")]
    pub detach: bool,
}

pub async fn execute(args: SwitchArgs) {
    // check status
    if check_status().await {
        return;
    }

    match args.create {
        Some(new_branch_name) => {
            branch::create_branch(new_branch_name.clone(), args.branch).await;
            switch_to_branch(new_branch_name).await;
        }
        None => match args.detach {
            true => {
                let commit_base = get_commit_base(&args.branch.unwrap()).await;
                if commit_base.is_err() {
                    eprintln!("{:?}", commit_base.unwrap_err());
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
        status::execute(StatusArgs::default()).await;
        eprintln!("fatal: unstaged changes, can't switch branch");
        true
    } else if !status::changes_to_be_committed().await.is_empty() {
        status::execute(StatusArgs::default()).await;
        eprintln!("fatal: uncommitted changes, can't switch branch");
        true
    } else {
        false
    }
}

/// change the working directory to the version of commit_hash
async fn switch_to_commit(commit_hash: SHA1) {
    let db = get_db_conn_instance().await;

    let old_head_commit = Head::current_commit_with_conn(db)
        .await
        .expect("Cannot switch: HEAD is unborn.");

    let from_ref_name = match Head::current_with_conn(db).await {
        Head::Branch(name) => name,
        Head::Detached(hash) => hash.to_string()[..7].to_string(), // Use short hash for detached HEAD
    };

    let action = ReflogAction::Switch {
        from: from_ref_name,
        to: commit_hash.to_string()[..7].to_string(), // Use short hash for target commit
    };
    let context = ReflogContext {
        old_oid: old_head_commit.to_string(),
        new_oid: commit_hash.to_string(),
        action,
    };

    if let Err(e) = with_reflog(
        context,
        move |txn: &sea_orm::DatabaseTransaction| {
            Box::pin(async move {
                let new_head = Head::Detached(commit_hash);
                Head::update_with_conn(txn, new_head, None).await;
                Ok(())
            })
        },
        false,
    )
    .await
    {
        eprintln!("fatal: {e}");
        return;
    };

    // Only restore the working directory *after* HEAD has been successfully updated.
    restore_to_commit(commit_hash).await;
    println!("HEAD is now at {}", &commit_hash.to_string()[..7]);
}

async fn switch_to_branch(branch_name: String) {
    let db = get_db_conn_instance().await;

    let target_branch = match Branch::find_branch_with_conn(db, &branch_name, None).await {
        Some(b) => b,
        None => {
            if !Branch::search_branch(&branch_name).await.is_empty() {
                eprintln!("fatal: a branch is expected, got remote branch {branch_name}");
            } else {
                eprintln!("fatal: branch '{}' not found", &branch_name);
            }
            return;
        }
    };
    let target_commit_id = target_branch.commit;

    let old_head_commit = Head::current_commit_with_conn(db)
        .await
        .expect("Cannot switch: HEAD is unborn.");

    let from_ref_name = match Head::current_with_conn(db).await {
        Head::Branch(name) => name,
        Head::Detached(hash) => hash.to_string()[..7].to_string(),
    };

    if from_ref_name == branch_name {
        println!("Already on '{branch_name}'");
        return;
    }

    let action = ReflogAction::Switch {
        from: from_ref_name,
        to: branch_name.clone(),
    };
    let context = ReflogContext {
        old_oid: old_head_commit.to_string(),
        new_oid: target_commit_id.to_string(),
        action,
    };

    // `log_for_branch` is `false`. This is the key insight for `switch`/`checkout`.
    if let Err(e) = with_reflog(
        context,
        move |txn: &sea_orm::DatabaseTransaction| {
            Box::pin(async move {
                let new_head = Head::Branch(branch_name.clone());
                Head::update_with_conn(txn, new_head, None).await;
                Ok(())
            })
        },
        false,
    )
    .await
    {
        eprintln!("fatal: {e}");
        return;
    }

    restore_to_commit(target_commit_id).await;
    println!("Switched to branch '{}'", target_branch.name);
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
    use crate::command::restore::RestoreArgs;
    use std::str::FromStr;
    #[test]
    /// Test parsing RestoreArgs from command-line style arguments
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
        println!("{restore_args:?}");
    }
}
