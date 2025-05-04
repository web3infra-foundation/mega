use clap::Parser;
use mercury::internal::object::commit::Commit;

use crate::{
    internal::{branch::Branch, head::Head},
    utils::util,
};

use super::{
    get_target_commit, load_object, log,
    restore::{self, RestoreArgs},
};

#[derive(Parser, Debug)]
pub struct MergeArgs {
    /// The branch to merge into the current branch, could be remote branch
    pub branch: String,
    /// The commit message for the merge commit
    #[arg(short = 'm', long, required = false)]
    pub message: Option<String>,
}

pub async fn execute(args: MergeArgs) {
    let target_commit_hash = get_target_commit(&args.branch).await;
    let merge_message = args
        .message
        .unwrap_or_else(|| format!("Merge branch '{}' into current", &args.branch));
    // Get the merge commit message.
    if target_commit_hash.is_err() {
        eprintln!("{}", target_commit_hash.err().unwrap());
        return;
    }
    let commit_hash = target_commit_hash.unwrap();
    let target_commit: Commit = load_object(&commit_hash).unwrap();
    let current_commit: Commit = load_object(&Head::current_commit().await.unwrap()).unwrap();
    let lca = lca_commit(&current_commit, &target_commit).await;
    if lca.is_none() {
        eprintln!("fatal: fatal: refusing to merge unrelated histories");
        return;
    }
    let mut lca = lca.unwrap();
    lca.message = merge_message;
    if lca.id == target_commit.id {
        // no need to merge
        println!("Already up to date.");
    } else if lca.id == current_commit.id {
        println!(
            "Updating {}..{}",
            &current_commit.id.to_string()[..6],
            &target_commit.id.to_string()[..6]
        );
        // fast-forward merge
        merge_ff(target_commit).await;
    } else {
        // didn't support yet
        eprintln!("fatal: Not possible to fast-forward merge, try merge manually");
    }
}

async fn lca_commit(lhs: &Commit, rhs: &Commit) -> Option<Commit> {
    let lhs_reachable = log::get_reachable_commits(lhs.id.to_string()).await;
    let rhs_reachable = log::get_reachable_commits(rhs.id.to_string()).await;
    // Commit `eq` is based on tree_id, so we shouldn't use it here
    for commit in lhs_reachable.iter() {
        if commit.id == rhs.id {
            return Some(commit.to_owned());
        }
    }
    for commit in rhs_reachable.iter() {
        if commit.id == lhs.id {
            return Some(commit.to_owned());
        }
    }
    for lhs_parent in lhs_reachable.iter() {
        for rhs_parent in rhs_reachable.iter() {
            if lhs_parent.id == rhs_parent.id {
                return Some(lhs_parent.to_owned());
            }
        }
    }
    None
}

/// try merge in fast-forward mode, if it's not possible, do nothing
async fn merge_ff(commit: Commit) {
    println!("Fast-forward");
    // fast-forward merge
    let head = Head::current().await;
    match head {
        Head::Branch(branch_name) => {
            Branch::update_branch(&branch_name, &commit.id.to_string(), None).await;
        }
        Head::Detached(_) => {
            Head::update(Head::Detached(commit.id), None).await;
        }
    }
    // change the working directory to the commit
    // restore all files to worktree from HEAD
    restore::execute(RestoreArgs {
        worktree: true,
        staged: true,
        source: None,
        pathspec: vec![util::working_dir_string()],
    })
    .await;
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    struct TestEnv {
        last_merge_message: Arc<Mutex<Option<String>>>,
    }

    impl TestEnv {
        fn new() -> Self {
            Self {
                last_merge_message: Arc::new(Mutex::new(None)),
            }
        }

        //Simulate the execution environment of execute
        async fn mock_execute(&self, args: MergeArgs) {
            let message = args
                .message
                .unwrap_or_else(|| format!("Merge branch '{}' into current", &args.branch));
            *self.last_merge_message.lock().unwrap() = Some(message);
        }
    }

    #[tokio::test]
    async fn test_merge_with_custom_message() {
        let env = TestEnv::new();

        // Test with -m parameter
        let args = MergeArgs {
            branch: "feature".to_string(),
            message: Some("Custom merge message".to_string()),
        };

        env.mock_execute(args).await;
        assert_eq!(
            *env.last_merge_message.lock().unwrap(),
            Some("Custom merge message".to_string())
        );
    }

    #[tokio::test]
    async fn test_merge_with_default_message() {
        let env = TestEnv::new();

        // Test without -m parameter
        let args = MergeArgs {
            branch: "hotfix".to_string(),
            message: None,
        };

        env.mock_execute(args).await;
        assert_eq!(
            *env.last_merge_message.lock().unwrap(),
            Some("Merge branch 'hotfix' into current".to_string())
        );
    }
}
