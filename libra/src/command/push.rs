use crate::command::branch;
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;
use crate::internal::protocol::https_client::HttpsClient;
use crate::internal::protocol::lfs_client::LFSClient;
use crate::internal::protocol::ProtocolClient;
use crate::utils::object_ext::{BlobExt, CommitExt, TreeExt};
use bytes::BytesMut;
use ceres::protocol::smart::{add_pkt_line_string, read_pkt_line};
use ceres::protocol::ServiceType::ReceivePack;
use clap::Parser;
use colored::Colorize;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use mercury::internal::pack::encode::PackEncoder;
use mercury::internal::pack::entry::Entry;
use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::str::FromStr;
use tokio::sync::mpsc;
use url::Url;

#[derive(Parser, Debug)]
pub struct PushArgs {
    // TODO --force
    /// repository, e.g. origin
    #[clap(requires("refspec"))]
    repository: Option<String>,
    /// ref to push, e.g. master
    #[clap(requires("repository"))]
    refspec: Option<String>,

    #[clap(long, short = 'u', requires("refspec"), requires("repository"))]
    set_upstream: bool,
}

pub async fn execute(args: PushArgs) {
    if args.repository.is_some() ^ args.refspec.is_some() {
        // must provide both or none
        eprintln!("fatal: both repository and refspec should be provided");
        return;
    }
    if args.set_upstream && args.refspec.is_none() {
        eprintln!("fatal: --set-upstream requires a branch name");
        return;
    }

    let branch = match Head::current().await {
        Head::Branch(name) => name,
        Head::Detached(_) => panic!("fatal: HEAD is detached while pushing"),
    };

    let repository = match args.repository {
        Some(repo) => repo,
        None => {
            // e.g. [branch "master"].remote = origin
            let remote = Config::get_remote(&branch).await;
            if let Some(remote) = remote {
                remote
            } else {
                eprintln!("fatal: no remote configured for branch '{branch}'");
                return;
            }
        }
    };
    let repo_url = Config::get_remote_url(&repository).await;

    let branch = args.refspec.unwrap_or(branch);
    let commit_hash = Branch::find_branch(&branch, None)
        .await
        .unwrap()
        .commit
        .to_string();

    println!("pushing {branch}({commit_hash}) to {repository}({repo_url})");

    let url = Url::parse(&repo_url).unwrap();
    let client = HttpsClient::from_url(&url);
    let refs = match client.discovery_reference(ReceivePack).await {
        Ok(refs) => refs,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };

    let tracked_branch = Config::get("branch", Some(&branch), "merge")
        .await // New branch may not have tracking branch
        .unwrap_or_else(|| format!("refs/heads/{branch}"));

    let tracked_ref = refs.iter().find(|r| r._ref == tracked_branch);
    // [0; 20] if new branch
    let remote_hash = tracked_ref
        .map(|r| r._hash.clone())
        .unwrap_or(SHA1::default().to_string());
    if remote_hash == commit_hash {
        println!("Everything up-to-date");
        return;
    }

    let mut data = BytesMut::new();
    add_pkt_line_string(
        &mut data,
        format!("{remote_hash} {commit_hash} {tracked_branch}\0report-status\n"),
    );
    data.extend_from_slice(b"0000");
    tracing::debug!("{:?}", data);

    // TODO 考虑remote有多个refs，可以少发一点commits
    let objs = incremental_objs(
        SHA1::from_str(&commit_hash).unwrap(),
        SHA1::from_str(&remote_hash).unwrap(),
    );

    {
        // upload lfs files
        let client = LFSClient::from_url(&url);
        let res = client.push_objects(&objs).await;
        if res.is_err() {
            eprintln!("fatal: LFS files upload failed, stop pushing");
            return;
        }
    }

    // let (tx, rx) = mpsc::channel::<Entry>();
    let (entry_tx, entry_rx) = mpsc::channel(1_000_000);
    let (stream_tx, mut stream_rx) = mpsc::channel(1_000_000);

    let encoder = PackEncoder::new(objs.len(), 0, stream_tx); // TODO: diff slow, so window_size = 0
    encoder.encode_async(entry_rx).await.unwrap();

    for entry in objs {
        // TODO progress bar
        entry_tx.send(entry).await.unwrap();
    }
    drop(entry_tx);

    println!("Compression...");
    let mut pack_data = Vec::new();
    while let Some(chunk) = stream_rx.recv().await {
        pack_data.extend(chunk);
    }
    data.extend_from_slice(&pack_data);
    println!("Delta compression done.");

    let res = client.send_pack(data.freeze()).await.unwrap(); // TODO: send stream

    if res.status() != 200 {
        eprintln!("status code: {}", res.status());
    }
    let mut data = res.bytes().await.unwrap();
    let (_, pkt_line) = read_pkt_line(&mut data);
    if pkt_line != "unpack ok\n" {
        eprintln!("fatal: unpack failed");
        return;
    }
    let (_, pkt_line) = read_pkt_line(&mut data);
    if !pkt_line.starts_with("ok".as_ref()) {
        eprintln!("fatal: ref update failed [{pkt_line:?}]");
        return;
    }
    let (len, _) = read_pkt_line(&mut data);
    assert_eq!(len, 0);

    println!("{}", "Push success".green());

    // set after push success
    if args.set_upstream {
        branch::set_upstream(&branch, &format!("{repository}/{branch}")).await;
    }
}

/// collect all commits from `commit_id` to root commit
fn collect_history_commits(commit_id: &SHA1) -> HashSet<SHA1> {
    if commit_id == &SHA1::default() {
        // 0000...0000 means not exist
        return HashSet::new();
    }

    let mut commits = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(*commit_id);
    while let Some(commit) = queue.pop_front() {
        commits.insert(commit);

        let commit = Commit::load(&commit);
        for parent in commit.parent_commit_ids.iter() {
            queue.push_back(*parent);
        }
    }
    commits
}

fn incremental_objs(local_ref: SHA1, remote_ref: SHA1) -> HashSet<Entry> {
    tracing::debug!("local_ref: {}, remote_ref: {}", local_ref, remote_ref);

    // just fast-forward optimization
    if remote_ref != SHA1::default() {
        // remote exists
        let mut commit = Commit::load(&local_ref);
        let mut commits = Vec::new();
        let mut ok = true;
        loop {
            commits.push(commit.id);
            if commit.id == remote_ref {
                break;
            }
            if commit.parent_commit_ids.len() != 1 {
                // merge commit or root commit
                ok = false;
                break;
            }
            // update commit to it's only parent
            commit = Commit::load(&commit.parent_commit_ids[0]);
        }
        if ok {
            // fast-forward
            let mut objs = HashSet::new();
            commits.reverse(); // from old to new
            for i in 0..commits.len() - 1 {
                let old_tree = Commit::load(&commits[i]).tree_id;
                let new_commit = Commit::load(&commits[i + 1]);
                objs.extend(diff_tree_objs(Some(&old_tree), &new_commit.tree_id));
                objs.insert(new_commit.into());
            }
            return objs;
        }
    }

    let mut objs = HashSet::new();
    let mut visit = HashSet::new(); // avoid duplicate commit visit
    let exist_commits = collect_history_commits(&remote_ref);
    let mut queue = VecDeque::new();
    if !exist_commits.contains(&local_ref) {
        queue.push_back(local_ref);
        visit.insert(local_ref);
    }
    let mut root_commit = None;

    while let Some(commit) = queue.pop_front() {
        let commit = Commit::load(&commit);
        let parents = &commit.parent_commit_ids;
        if parents.is_empty() {
            if root_commit.is_none() {
                root_commit = Some(commit.id);
            } else if root_commit != Some(commit.id) {
                eprintln!("{}", "fatal: multiple root commits".red());
            }
        }
        for parent in parents.iter() {
            let parent_tree = Commit::load(parent).tree_id;
            objs.extend(diff_tree_objs(Some(&parent_tree), &commit.tree_id));
            if !exist_commits.contains(parent) && !visit.contains(parent) {
                queue.push_back(*parent);
                visit.insert(*parent);
            }
        }
        objs.insert(commit.into());

        print!("Counting objects: {}\r", objs.len());
        std::io::stdout().flush().unwrap();
    }

    // root commit has no parent
    if let Some(root_commit) = root_commit {
        let root_tree = Commit::load(&root_commit).tree_id;
        objs.extend(diff_tree_objs(None, &root_tree));
    }

    println!("Counting objects: {} done.", objs.len());
    objs
}

/// calc objects that in `new_tree` but not in `old_tree`
/// - if `old_tree` is None, return all objects in `new_tree` (include tree itself)
fn diff_tree_objs(old_tree: Option<&SHA1>, new_tree: &SHA1) -> HashSet<Entry> {
    // TODO: skip objs that has been added in caller
    let mut objs = HashSet::new();
    if let Some(old_tree) = old_tree {
        if old_tree == new_tree {
            return objs;
        }
    }

    let new_tree = Tree::load(new_tree);
    objs.insert(new_tree.clone().into()); // tree itself

    let old_items = match old_tree {
        Some(tree) => {
            let tree = Tree::load(tree);
            tree.tree_items
                .iter()
                .map(|item| item.id)
                .collect::<HashSet<_>>()
        }
        None => HashSet::new(),
    };

    for item in new_tree.tree_items.iter() {
        if !old_items.contains(&item.id) {
            match item.mode {
                TreeItemMode::Tree => {
                    objs.extend(diff_tree_objs(None, &item.id)); //TODO optimize, find same name tree
                }
                _ => {
                    // TODO: submodule (TreeItemMode: Commit)
                    if item.mode == TreeItemMode::Commit {
                        // (160000)| Gitlink (Submodule)
                        eprintln!("{}", "Warning: Submodule is not supported yet".red());
                    }
                    let blob = Blob::load(&item.id);
                    objs.insert(blob.into());
                }
            }
        }
    }

    objs
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    /// Tests successful parsing of push command arguments with different parameter combinations.
    /// Verifies repository, refspec and upstream flag settings are correctly interpreted.
    fn test_parse_args_success() {
        let args = vec!["push"];
        let args = PushArgs::parse_from(args);
        assert_eq!(args.repository, None);
        assert_eq!(args.refspec, None);
        assert!(!args.set_upstream);

        let args = vec!["push", "origin", "master"];
        let args = PushArgs::parse_from(args);
        assert_eq!(args.repository, Some("origin".to_string()));
        assert_eq!(args.refspec, Some("master".to_string()));
        assert!(!args.set_upstream);

        let args = vec!["push", "-u", "origin", "master"];
        let args = PushArgs::parse_from(args);
        assert_eq!(args.repository, Some("origin".to_string()));
        assert_eq!(args.refspec, Some("master".to_string()));
        assert!(args.set_upstream);
    }

    #[test]
    /// Tests failure cases for push command argument parsing with invalid parameter combinations.
    /// Verifies that missing required parameters are properly detected as errors.
    fn test_parse_args_fail() {
        let args = vec!["push", "-u"];
        let args = PushArgs::try_parse_from(args);
        assert!(args.is_err());

        let args = vec!["push", "-u", "origin"];
        let args = PushArgs::try_parse_from(args);
        assert!(args.is_err());

        let args = vec!["push", "-u", "master"];
        let args = PushArgs::try_parse_from(args);
        assert!(args.is_err());

        let args = vec!["push", "origin"];
        let args = PushArgs::try_parse_from(args);
        assert!(args.is_err());
    }
}
