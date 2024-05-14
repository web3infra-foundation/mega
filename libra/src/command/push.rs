use std::collections::{HashSet, VecDeque};
use std::fs;
use std::hash::Hash;
use std::io::Write;
use std::str::FromStr;
use std::sync::mpsc;
use bytes::BytesMut;
use clap::Parser;
use url::Url;
use ceres::protocol::ServiceType::ReceivePack;
use ceres::protocol::smart::add_pkt_line_string;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use mercury::internal::object::blob::Blob;
use mercury::internal::object::commit::Commit;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use mercury::internal::object::types::ObjectType;
use mercury::internal::pack::encode::PackEncoder;
use mercury::internal::pack::entry::Entry;
use crate::command::ask_username_password;
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;
use crate::internal::protocol::https_client::HttpsClient;
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
    let repo_url = Config::get("remote", Some(&repository), "url").await.unwrap();

    let branch = args.refspec.unwrap_or(branch);
    let commit_hash = Branch::find_branch(&branch, None).await.unwrap().commit.to_plain_str();

    println!("pushing {}({}) to {}({})", branch, commit_hash, repository, repo_url);

    let url = Url::parse(&repo_url).unwrap();
    let client = HttpsClient::from_url(&url);
    let mut refs = client.discovery_reference(ReceivePack, None).await;
    let mut auth: Option<(String, String)> = None;
    while let Err(e) = refs { // retry if unauthorized
        if let GitError::UnAuthorized(_) = e {
            let (username, password) = ask_username_password();
            auth = Some((username.clone(), password.clone()));
            refs = client.discovery_reference(ReceivePack, Some((username, Some(password)))).await;
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

    let mut encoder = PackEncoder::new(objs.len(), 5);
    let (tx, rx) = mpsc::channel::<Entry>();
    for entry in objs {
        println!("{:?}", entry.hash.to_plain_str());
        tx.send(entry).unwrap();
    }
    drop(tx);
    let pack_data = encoder.encode(rx).unwrap();
    println!("pack data len: {:?}", pack_data.len());
    fs::File::create("/tmp/tmpPack.pack").unwrap().write_all(&pack_data).unwrap();

    data.extend_from_slice(&pack_data);

    let (username, password) = auth.unwrap();

    let res = client
        .client
        .post(client.url.join("git-receive-pack").unwrap())
        .header("Content-Type", "application/x-git-receive-pack-request")
        .basic_auth(username, Some(password))
        .body(data.freeze())
        .send()
        .await
        .unwrap();
    println!("{:?}", res);
    println!("{:?}", res.bytes().await.unwrap());
}

fn objs_between_commits(local_commit: SHA1, remote_commit: SHA1) -> HashSet<Entry> {
    let mut objs = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(local_commit.clone());

    let mut last_commit = local_commit.clone();
    while let Some(commit) = queue.pop_front() {
        if commit != last_commit {
            let old_commit = Commit::load(&commit);
            let new_commit = Commit::load(&last_commit);
            objs.extend(diff_tree_objs(Some(&old_commit.tree_id), &new_commit.tree_id));
            objs.insert(new_commit.into()); // commit itself
        }
        last_commit = commit.clone();
        if commit == remote_commit {
            break;
        }

        let commit = Commit::load(&commit);
        for parent in commit.parent_commit_ids.iter() {
            queue.push_back(*parent);
        }
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