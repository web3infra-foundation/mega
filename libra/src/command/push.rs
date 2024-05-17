use std::collections::{HashSet, VecDeque};
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
use crate::command::{ask_basic_auth, branch};
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;
use crate::internal::protocol::https_client::{BasicAuth, HttpsClient};
use crate::internal::protocol::ProtocolClient;
use crate::utils::object_ext::{BlobExt, CommitExt, TreeExt};

#[derive(Parser, Debug)]
pub struct PushArgs { // TODO --force
    /// repository, e.g. origin
    repository: Option<String>,
    /// ref to push, e.g. master
    refspec: Option<String>,

    #[clap(long, short = 'u')]
    set_upstream: bool,
}

pub async fn execute(args: PushArgs) {
    if args.repository.is_some() ^ args.refspec.is_some() { // must provide both or none
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
            let remote = Config::get("branch", Some(&branch), "remote").await;
            if let Some(remote) = remote {
                remote
            } else {
                eprintln!("fatal: no remote configured for branch '{}'", branch);
                return;
            }
        }
    };
    let repo_url = Config::get("remote", Some(&repository), "url").await;
    if repo_url.is_none() {
        eprintln!("fatal: remote '{}' not found, please use 'libra remote add'", repository);
        return;
    }
    let repo_url = repo_url.unwrap();

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
    tracing::debug!("{:?}", data);

    // TODO 考虑remote有多个refs，可以少发一点commits
    let objs = incremental_objs(
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

    // set after push success
    if args.set_upstream {
        branch::set_upstream(&branch, &format!("{}/{}", repository, branch)).await;
    }
}

/// collect all commits from `commit_id` to root commit
fn collect_history_commits(commit_id: &SHA1) -> HashSet<SHA1> {
    if commit_id == &SHA1::default() { // 0000...0000 means not exist
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
    // just fast-forward optimization
    if remote_ref != SHA1::default() { // remote exists
        let mut commit = Commit::load(&local_ref);
        let mut commits = Vec::new();
        let mut ok = true;
        loop {
            commits.push(commit.id);
            if commit.id == remote_ref {
                break;
            }
            if commit.parent_commit_ids.len() != 1 { // merge commit or root commit
                ok = false;
                break;
            }
            // update commit to it's only parent
            commit = Commit::load(&commit.parent_commit_ids[0]);
        }
        if ok { // fast-forward
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
    let exist_commits = collect_history_commits(&remote_ref);
    let mut queue = VecDeque::new();
    if !exist_commits.contains(&local_ref) {
        queue.push_back(local_ref);
    }

    while let Some(commit) = queue.pop_front() {
        let commit = Commit::load(&commit);
        for parent in commit.parent_commit_ids.iter() {
            objs.extend(diff_tree_objs(Some(parent), &commit.tree_id));
            if !exist_commits.contains(parent) {
                queue.push_back(*parent);
            }
        }
        objs.insert(commit.into());
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

    let new_tree = Tree::load(new_tree);
    objs.insert(new_tree.clone().into()); // tree itself

    let old_items = match old_tree {
        Some(tree) => {
            let tree = Tree::load(tree);
            tree.tree_items.iter()
                .map(|item| item.id)
                .collect::<HashSet<_>>()
        }
        None => HashSet::new()
    };

    for item in new_tree.tree_items.iter() {
        if !old_items.contains(&item.id) {
            match item.mode {
                TreeItemMode::Tree => {
                    objs.extend(diff_tree_objs(None, &item.id)); //TODO optimize, find same name tree
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