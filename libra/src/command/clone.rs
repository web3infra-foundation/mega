use crate::command::{self, branch};
use crate::command::restore::RestoreArgs;
use crate::internal::branch::Branch;
use crate::internal::config::{Config, RemoteConfig};
use crate::internal::head::Head;
use crate::utils::path_ext::PathExt;
use crate::utils::util;
use clap::Parser;
use colored::Colorize;
use scopeguard::defer;
use std::cell::Cell;
use std::path::PathBuf;
use std::{env, fs};

use super::fetch::{self};

const ORIGIN: &str = "origin"; // default remote name, prevent spelling mistakes

#[derive(Parser, Debug)]
pub struct CloneArgs {
    /// The remote repository location to clone from, usually a URL with HTTPS or SSH
    pub remote_repo: String,

    /// The local path to clone the repository to
    pub local_path: Option<String>,

    /// The branch to clone
    #[clap(short = 'b', long, required = false)]
    pub branch: Option<String>,
}

pub async fn execute(args: CloneArgs) {
    let mut remote_repo = args.remote_repo; // https://gitee.com/caiqihang2024/image-viewer2.0.git
                                            // must end with '/' or Url::join will work incorrectly
    if !remote_repo.ends_with('/') {
        remote_repo.push('/');
    }
    let local_path = args.local_path.unwrap_or_else(|| {
        let repo_name = util::get_repo_name_from_url(&remote_repo).unwrap();
        util::cur_dir().join(repo_name).to_string_or_panic()
    });

    /* create local path */
    let local_path = PathBuf::from(local_path);
    {
        if local_path.exists() && !util::is_empty_dir(&local_path) {
            eprintln!(
                "fatal: destination path '{}' already exists and is not an empty directory.",
                local_path.display()
            );
            return;
        }

        // make sure the directory exists
        if let Err(e) = fs::create_dir_all(&local_path) {
            eprintln!(
                "fatal: could not create directory '{}': {}",
                local_path.display(),
                e
            );
            return;
        }
        let repo_name = local_path.file_name().unwrap().to_str().unwrap();
        println!("Cloning into '{}'", repo_name);
    }

    let is_success = Cell::new(false);
    // clean up the directory if panic
    defer! {
        if !is_success.get() {
            fs::remove_dir_all(&local_path).unwrap();
            eprintln!("{}", "fatal: clone failed, delete repo directory automatically".red());
        }
    }

    //check if the branch name is valid
    if let Some(branch) = args.branch.clone() {
        if !branch::is_valid_git_branch_name(&branch) {
            eprintln!("invalid branch name: '{}'.\n\nBranch names must:\n- Not contain spaces, control characters, or any of these characters: \\ : \" ? * [\n- Not start or end with a slash ('/'), or end with a dot ('.')\n- Not contain consecutive slashes ('//') or dots ('..')\n- Not be reserved names like 'HEAD' or contain '@{{'\n- Not be empty or just a dot ('.')\n\nPlease choose a valid branch name.", branch);
            return;
        }
    }

    // CAUTION: change [current_dir] to the repo directory
    env::set_current_dir(&local_path).unwrap();
    let init_args = command::init::InitArgs {
        bare: false,
        initial_branch: args.branch.clone(),
        repo_directory: local_path.to_str().unwrap().to_string(),
        quiet: false,
    };
    command::init::execute(init_args).await;

    /* fetch remote */
    let remote_config = RemoteConfig {
        name: "origin".to_string(),
        url: remote_repo.clone(),
    };
    fetch::fetch_repository(&remote_config, args.branch.clone()).await;

    /* setup */
    setup(remote_repo.clone(), args.branch.clone()).await;

    is_success.set(true);
}

async fn setup(remote_repo: String, specified_branch: Option<String>) {
    // look for remote head and set local HEAD&branch
    let remote_head = Head::remote_current(ORIGIN).await;

    if let Some(specified_branch) = specified_branch {
        setup_branch(specified_branch).await;
    }else if let Some(Head::Branch(name)) = remote_head {
        setup_branch(name).await;
    }else if let Some(Head::Detached(_)) = remote_head {
        eprintln!("fatal: remote HEAD points to a detached commit");
    }else {
        println!("warning: You appear to have cloned an empty repository.");

        // set config: remote.origin.url
        Config::insert("remote", Some(ORIGIN), "url", &remote_repo).await;
        // set config: remote.origin.fetch
        // todo: temporary ignore fetch option

        // set config: branch.$name.merge, e.g.
        let merge = "refs/heads/master".to_owned();
        Config::insert("branch", Some("master"), "merge", &merge).await;
        // set config: branch.$name.remote
        Config::insert("branch", Some("master"), "remote", ORIGIN).await;
    }
}

async fn setup_branch(branch_name: String) {
    let origin_head_branch = Branch::find_branch(&branch_name, Some(ORIGIN))
    .await
    .expect("origin HEAD branch not found");

    Branch::update_branch(&branch_name, &origin_head_branch.commit.to_string(), None).await;
    Head::update(Head::Branch(branch_name.to_owned()), None).await;

    let merge = "refs/heads/".to_owned() + &branch_name;
    Config::insert("branch", Some(&branch_name), "merge", &merge).await;
    Config::insert("branch", Some(&branch_name), "remote", ORIGIN).await;

    command::restore::execute(RestoreArgs {
        worktree: true,
        staged: true,
        source: None,
        pathspec: vec![util::working_dir_string()],
    })
    .await;
}

/// Unit tests for the clone module
#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::tempdir;
    use std::path::Path;
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_clone_branch() {
        let local_dir = tempdir().unwrap().into_path();
        let local_repo = local_dir.to_str().unwrap().to_string();

        let remote_url = "https://gitee.com/pikady/mega-libra-clone-branch-test.git".to_string();
        
        command::clone::execute(CloneArgs {
            remote_repo: remote_url,
            local_path: Some(local_repo.clone()),
            branch: Some("dev".to_string()),
        }).await;

        // Verify that the `.libra` directory exists
        let libra_dir = Path::new(&local_repo).join(".libra");
        assert!(libra_dir.exists());

        // Verify the Head reference
        match Head::current().await {
            Head::Branch(current_branch) => {
                assert_eq!(current_branch, "dev");
            }
            _ => panic!("should be branch"),
        };
    }

    #[tokio::test]
    #[serial]
    async fn test_clone_default_branch() {
        let local_dir = tempdir().unwrap().into_path();
        let local_repo = local_dir.to_str().unwrap().to_string();

        let remote_url = "https://gitee.com/pikady/mega-libra-clone-branch-test.git".to_string();
        
        command::clone::execute(CloneArgs {
            remote_repo: remote_url,
            local_path: Some(local_repo.clone()),
            branch: None,
        }).await;

        // Verify that the `.libra` directory exists
        let libra_dir = Path::new(&local_repo).join(".libra");
        assert!(libra_dir.exists());

        // Verify the Head reference
        match Head::current().await {
            Head::Branch(current_branch) => {
                assert_eq!(current_branch, "master");
            }
            _ => panic!("should be branch"),
        };
    }

    #[tokio::test]
    #[serial]
    async fn test_clone_empty_repo() {
        let local_dir = tempdir().unwrap().into_path();
        let local_repo = local_dir.to_str().unwrap().to_string();

        let remote_url = "https://gitee.com/pikady/mega-libra-empty-repo.git".to_string();
        
        command::clone::execute(CloneArgs {
            remote_repo: remote_url,
            local_path: Some(local_repo.clone()),
            branch: None,
        }).await;

        // Verify that the `.libra` directory exists
        let libra_dir = Path::new(&local_repo).join(".libra");
        assert!(libra_dir.exists());

        // Verify the Head reference
        match Head::current().await {
            Head::Branch(current_branch) => {
                assert_eq!(current_branch, "master");
            }
            _ => panic!("should be branch"),
        };
    }
}
