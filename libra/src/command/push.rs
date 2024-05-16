use std::collections::{HashMap, HashSet, VecDeque};
use std::str::FromStr;
use std::sync::mpsc;
use bytes::BytesMut;
use clap::Parser;
use url::Url;
use ceres::protocol::ServiceType::ReceivePack;
use ceres::protocol::smart::{add_pkt_line_string, read_pkt_line};
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use mercury::internal::pack::encode::PackEncoder;
use mercury::internal::pack::entry::Entry;
use crate::command::ask_basic_auth;
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;
use crate::internal::protocol::https_client::{BasicAuth, HttpsClient};
use crate::internal::protocol::ProtocolClient;
use crate::utils::object_ext::{BlobExt, CommitExt, TreeExt};

#[derive(Parser, Debug)]
pub struct PushArgs {
    /// repository, e.g. origin
    repository: Option<String>,
    /// ref to push, e.g. master
    refspec: Option<String>,
}

#[allow(unused_variables)]
pub async fn execute(args: PushArgs) {
    let branch = match Head::current().await {
        Head::Branch(name) => name,
        Head::Detached(_) => panic!("fatal: HEAD is detached while pushing"),
    };

    let repository = match args.repository {
        Some(repo) => repo,
        None => {
            // e.g. [branch "master"].remote = origin
            Config::get("branch", Some(&branch), "remote").await.unwrap()
        }
    };
    let repo_url = Config::get("remote", Some(&repository), "url").await
        .unwrap_or("https://gitee.com/caiqihang2024/test-git-remote-2.git".to_string()); // TODO remote command

    let branch = args.refspec.unwrap_or(branch);
    let commit_hash = Branch::find_branch(&branch, None).await.unwrap().commit.to_plain_str();

    println!("pushing {}({}) to {}({})", branch, commit_hash, repository, repo_url);

    let url = Url::parse(&repo_url).unwrap();
    let client = HttpsClient::from_url(&url);
    let mut refs = client.discovery_reference(ReceivePack, None).await;
    let mut auth: Option<BasicAuth> = None;
    while let Err(e) = refs { // retry if unauthorized
        if let GitError::UnAuthorized(_) = e {
            auth = Some(ask_basic_auth());
            refs = client.discovery_reference(ReceivePack, auth.clone()).await;
        } else {
            eprintln!("fatal: {}", e);
            return;
        }
    }
    let refs = refs.unwrap();

    let tracked_branch = Config::get("branch", Some(&branch), "merge")
        .await // New branch may not have tracking branch
        .unwrap_or_else(|| format!("refs/heads/{}", branch));

    let tracked_ref = refs.iter().find(|r| r._ref == tracked_branch);
    // [0; 20] if new branch
    let remote_hash = tracked_ref.map(|r| r._hash.clone()).unwrap_or(SHA1::default().to_plain_str());
    if remote_hash == commit_hash {
        println!("Everything up-to-date");
        return;
    }

    let mut data = BytesMut::new();
    add_pkt_line_string(&mut data, format!("{} {} {}\0report-status\n",
                                           remote_hash,
                                           commit_hash,
                                           tracked_branch));
    data.extend_from_slice(b"0000");
    println!("{:?}", data);

    let objs = objs_between_commits(
        SHA1::from_str(&commit_hash).unwrap(),
        SHA1::from_str(&remote_hash).unwrap()
    );
    println!("Counting objects: {}", objs.len());

    let mut encoder = PackEncoder::new(objs.len(), 5);
    let (tx, rx) = mpsc::channel::<Entry>();
    for entry in objs {
        // TODO progress bar
        tx.send(entry).unwrap();
    }
    drop(tx);
    let pack_data = encoder.encode(rx).unwrap();
    println!("Delta compression done.");

    data.extend_from_slice(&pack_data);

    let res = client.send_pack(data.freeze(), auth).await.unwrap();

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
        eprintln!("fatal: ref update failed [{:?}]", pkt_line);
        return;
    }
    let (len, _) = read_pkt_line(&mut data);
    assert_eq!(len, 0);

    println!("Push success");
}

/// return commit paths from old_commit to new_commit
fn find_commit_paths(old_commit: &SHA1, new_commit: &SHA1) -> Vec<SHA1> {
    let mut child = HashMap::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(*new_commit);
    visited.insert(*new_commit);

    let mut last_commit = new_commit.clone();
    while let Some(commit) = queue.pop_front() {
        last_commit = commit.clone();
        if commit == *old_commit {
            break;
        }

        let commit = Commit::load(&commit);
        for parent in commit.parent_commit_ids.iter() {
            if !visited.contains(parent) {
                child.insert(*parent, commit.id);
                queue.push_back(*parent);
                visited.insert(*parent);
            }
        }
    }

    // found old_commit or got root commit(assume there is only one root commit)
    assert!(last_commit == *old_commit || Commit::load(&last_commit).parent_commit_ids.is_empty());

    let mut paths = Vec::new();
    let mut commit = last_commit;
    while commit != *new_commit {
        paths.push(commit);
        commit = child[&commit];
    }
    assert_eq!(commit, *new_commit);
    paths.push(commit);

    paths
}

fn objs_between_commits(local_commit: SHA1, remote_commit: SHA1) -> HashSet<Entry> {
    let mut objs = HashSet::new();
    let commits = find_commit_paths(&remote_commit, &local_commit);
    assert!(commits.len() > 0);
    if commits[0] != remote_commit {
        let root_commit = Commit::load(&commits[0]);
        objs.extend(diff_tree_objs(None, &root_commit.tree_id));
        objs.insert(root_commit.into());
    }

    for i in 0..commits.len() - 1 {
        let old_commit = commits[i];
        let new_commit = commits[i + 1];
        let old_tree = Commit::load(&old_commit).tree_id;
        let new_commit = Commit::load(&new_commit);
        objs.extend(diff_tree_objs(Some(&old_tree), &new_commit.tree_id));
        objs.insert(new_commit.into());
    }
    objs
}

/// calc objects that in `new_tree` but not in `old_tree`
/// - if `old_tree` is None, return all objects in `new_tree` (include tree itself)
fn diff_tree_objs(old_tree: Option<&SHA1>, new_tree: &SHA1) -> HashSet<Entry> {
    let mut objs = HashSet::new();
    if let Some(old_tree) = old_tree {
        if old_tree == new_tree {
            return objs;
        }
    }

    let new_tree = Tree::load(&new_tree);
    objs.insert(new_tree.clone().into()); // tree itself

    let old_items = match old_tree {
        Some(tree) => {
            let tree = Tree::load(tree);
            tree.tree_items.iter()
                .map(|item| item.id.clone())
                .collect::<HashSet<_>>()
        }
        None => HashSet::new()
    };

    for item in new_tree.tree_items.iter() {
        if !old_items.contains(&item.id) {
            match item.mode {
                TreeItemMode::Tree => {
                    objs.extend(diff_tree_objs(None, &item.id)); //TODO optimize
                }
                _ => {
                    let blob = Blob::load(&item.id);
                    objs.insert(blob.into());
                }
            }
        }
    }

    objs
}