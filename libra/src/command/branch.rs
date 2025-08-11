use crate::{
    command::get_target_commit,
    internal::{branch::Branch, config::Config, head::Head},
};
use clap::Parser;
use colored::Colorize;
use mercury::internal::object::commit::Commit;

use crate::command::load_object;

#[derive(Parser, Debug)]
pub struct BranchArgs {
    /// new branch name
    #[clap(group = "sub")]
    pub new_branch: Option<String>,

    /// base branch name or commit hash
    #[clap(requires = "new_branch")]
    pub commit_hash: Option<String>,

    /// list all branches, don't include remote branches
    #[clap(short, long, group = "sub", default_value = "true")]
    pub list: bool,

    /// force delete branch
    #[clap(short = 'D', long, group = "sub")]
    pub delete: Option<String>,

    ///  Set up `branchname`>`'s tracking information so `<`upstream`>` is considered `<`branchname`>`'s upstream branch.
    #[clap(short = 'u', long, group = "sub")]
    pub set_upstream_to: Option<String>,

    /// show current branch
    #[clap(long, group = "sub")]
    pub show_current: bool,

    /// show remote branches
    #[clap(short, long)] // TODO limit to required `list` option, even in default
    pub remotes: bool,
}
pub async fn execute(args: BranchArgs) {
    if args.new_branch.is_some() {
        create_branch(args.new_branch.unwrap(), args.commit_hash).await;
    } else if args.delete.is_some() {
        delete_branch(args.delete.unwrap()).await;
    } else if args.show_current {
        show_current_branch().await;
    } else if args.set_upstream_to.is_some() {
        match Head::current().await {
            Head::Branch(name) => set_upstream(&name, &args.set_upstream_to.unwrap()).await,
            Head::Detached(_) => {
                eprintln!("fatal: HEAD is detached");
            }
        };
    } else if args.list {
        // default behavior
        list_branches(args.remotes).await;
    } else {
        panic!("should not reach here")
    }
}

pub async fn set_upstream(branch: &str, upstream: &str) {
    let branch_config = Config::branch_config(branch).await;
    if branch_config.is_none() {
        let (remote, remote_branch) = match upstream.split_once('/') {
            Some((remote, branch)) => (remote, branch),
            None => {
                eprintln!("fatal: invalid upstream '{upstream}'");
                return;
            }
        };
        Config::insert("branch", Some(branch), "remote", remote).await;
        // set upstream branch (tracking branch)
        Config::insert(
            "branch",
            Some(branch),
            "merge",
            &format!("refs/heads/{remote_branch}"),
        )
        .await;
    }
    println!("Branch '{branch}' set up to track remote branch '{upstream}'");
}

pub async fn create_branch(new_branch: String, branch_or_commit: Option<String>) {
    tracing::debug!("create branch: {} from {:?}", new_branch, branch_or_commit);

    if !is_valid_git_branch_name(&new_branch) {
        eprintln!("fatal: invalid branch name: {new_branch}");
        return;
    }

    // check if branch exists
    let branch = Branch::find_branch(&new_branch, None).await;
    if branch.is_some() {
        panic!("fatal: A branch named '{new_branch}' already exists.");
    }

    let commit_id = match branch_or_commit {
        Some(branch_or_commit) => {
            let commit = get_target_commit(&branch_or_commit).await;
            match commit {
                Ok(commit) => commit,
                Err(e) => {
                    eprintln!("fatal: {e}");
                    return;
                }
            }
        }
        None => Head::current_commit().await.unwrap(),
    };
    tracing::debug!("base commit_id: {}", commit_id);

    // check if commit_hash exists
    let _ = load_object::<Commit>(&commit_id)
        .unwrap_or_else(|_| panic!("fatal: not a valid object name: '{commit_id}'"));

    // create branch
    Branch::update_branch(&new_branch, &commit_id.to_string(), None).await;
}

async fn delete_branch(branch_name: String) {
    let _ = Branch::find_branch(&branch_name, None)
        .await
        .unwrap_or_else(|| panic!("fatal: branch '{branch_name}' not found"));
    let head = Head::current().await;

    if let Head::Branch(name) = head {
        if name == branch_name {
            panic!("fatal: Cannot delete the branch '{branch_name}' which you are currently on");
        }
    }

    Branch::delete_branch(&branch_name, None).await;
}

async fn show_current_branch() {
    // let head = reference::Model::current_head(&db).await.unwrap();
    let head = Head::current().await;
    match head {
        Head::Detached(commit_hash) => {
            println!("HEAD detached at {}", &commit_hash.to_string()[..8]);
        }
        Head::Branch(name) => {
            println!("{name}");
        }
    }
}

pub async fn list_branches(remotes: bool) {
    let branches = match remotes {
        true => {
            // list all remote branches
            let remote_configs = Config::all_remote_configs().await;
            let mut branches = vec![];
            for remote in remote_configs {
                let remote_branches = Branch::list_branches(Some(&remote.name)).await;
                branches.extend(remote_branches);
            }
            branches
        }
        false => Branch::list_branches(None).await,
    };

    let head = Head::current().await;
    if let Head::Detached(commit) = head {
        let s = "HEAD detached at  ".to_string() + &commit.to_string()[..8];
        let s = s.green();
        println!("{s}");
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
            println!("  {name}");
        };
    }
}

pub fn is_valid_git_branch_name(name: &str) -> bool {
    // Validate branch name
    // Not contain spaces, control characters or special characters
    if name.contains(&[' ', '\t', '\\', ':', '"', '?', '*', '['][..])
        || name.chars().any(|c| c.is_ascii_control())
    {
        return false;
    }

    // Not start or end with a slash ('/'), or end with a dot ('.')
    // Not contain consecutive slashes ('//') or dots ('..')
    if name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with('.')
        || name.contains("//")
        || name.contains("..")
    {
        return false;
    }

    // Not be reserved names like 'HEAD' or contain '@{'
    if name == "HEAD" || name.contains("@{") {
        return false;
    }

    // Not be empty or just a dot ('.')
    if name.trim().is_empty() || name.trim() == "." {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {}
