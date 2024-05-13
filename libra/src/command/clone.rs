use std::path::PathBuf;
use std::{env, fs};

use crate::command;
use crate::command::restore::RestoreArgs;
use crate::internal::branch::Branch;
use crate::internal::config::{Config, RemoteConfig};
use crate::internal::head::Head;
use clap::Parser;

use crate::utils::path_ext::PathExt;
use crate::utils::util;

use super::fetch::{self};

#[derive(Parser, Debug)]
pub struct CloneArgs {
    /// The remote repository location to clone from, usually a URL with HTTPS or SSH
    pub remote_repo: String,

    /// The local path to clone the repository to
    pub local_path: Option<String>,
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

    // CAUTION: change [current_dir] to the repo directory
    env::set_current_dir(&local_path).unwrap();
    command::init::execute().await;

    /* fetch remote */
    let remote_config = RemoteConfig {
        name: "origin".to_string(),
        url: remote_repo.clone(),
    };
    fetch::fetch_repository(&remote_config).await;

    /* setup table */
    setup_head_and_config(remote_repo.clone()).await;

    // restore all files to worktree from HEAD
    command::restore::execute(RestoreArgs {
        worktree: true,
        staged: true,
        source: None,
        pathspec: vec![util::working_dir_string()],
    })
    .await;
}

async fn setup_head_and_config(remote_repo: String) {
    const ORIGIN: &str = "origin"; // default remote name, prevent spelling mistakes

    // let origin_head = Branch::find_branch("HEAD", Some(ORIGIN))
    //     .await
    //     .expect("origin HEAD not found");

    // Branch::update_branch(&origin_head_name, &head_ref._hash, None).await;
    // look for remote head and set local HEAD&branch
    let remote_head = Head::remote_current(ORIGIN)
        .await
        .expect("origin HEAD not found");

    match remote_head {
        Head::Branch(name) => {
            let origin_head_branch = Branch::find_branch(&name, Some(ORIGIN))
                .await
                .expect("origin HEAD branch not found");

            Branch::update_branch(&name, &origin_head_branch.commit.to_plain_str(), None).await;
            Head::update(Head::Branch(name.to_owned()), None).await;

            // set config: remote.origin.url
            Config::insert("remote", Some(ORIGIN), "url", &remote_repo).await;
            // set config: remote.origin.fetch
            // todo: temporary ignore fetch option

            // set config: branch.$name.merge, e.g.
            let merge = "refs/heads/".to_owned() + &name;
            Config::insert("branch", Some(&name), "merge", &merge).await;
            // set config: branch.$name.remote
            Config::insert("branch", Some(&name), "remote", ORIGIN).await;
        }
        Head::Detached(_) => {
            eprintln!("fatal: remote HEAD points to a detached commit");
        }
    }
}
