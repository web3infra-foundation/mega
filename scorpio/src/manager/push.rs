use bytes::BytesMut;
use ceres::protocol::smart::add_pkt_line_string;
use regex::Regex;
use reqwest::{header::CONTENT_TYPE, Client, Response, Url};
use std::io::{Error, ErrorKind};
use std::{path::Path, str::FromStr};
use tokio::sync::mpsc;

use crate::manager::store::{BlobFsStore, TreeStore};

use mercury::{
    hash::SHA1,
    internal::{
        object::{
            blob::Blob,
            commit::Commit,
            signature::{Signature, SignatureType},
            tree::Tree,
        },
        pack::encode::PackEncoder,
    },
};

pub async fn pack(commit: Commit, trees: Vec<Tree>, blob: Vec<Blob>) -> Vec<u8> {
    let len = trees.len() + blob.len() + 1;
    // let (tx, rx) = mpsc::channel::<Entry>();
    let (entry_tx, entry_rx) = mpsc::channel(1_000_000);
    let (stream_tx, mut stream_rx) = mpsc::channel(1_000_000);

    let encoder = PackEncoder::new(len, 0, stream_tx);
    encoder.encode_async(entry_rx).await.unwrap();
    entry_tx.send(commit.into()).await.unwrap();
    for v in trees {
        entry_tx.send(v.into()).await.unwrap();
    }
    for b in blob {
        entry_tx.send(b.into()).await.unwrap();
    }
    drop(entry_tx);

    println!("Compression...");
    let mut pack_data = Vec::new();
    while let Some(chunk) = stream_rx.recv().await {
        pack_data.extend(chunk);
    }
    pack_data
}

fn string_to_sha(hash: &str) -> std::io::Result<SHA1> {
    SHA1::from_str(hash).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

fn extract_commit_from_bytes(commitpath: &Path) -> std::io::Result<Commit> {
    let commit_string = std::fs::read_to_string(&commitpath)?;
    // This function uses regular expressions to extract the
    // required data, which may be split later to improve fault
    // tolerance.
    let regex_rule = Regex::new(
        r####"(?x)
    # trees' hash
    tree[^0-9a-z]+(?P<current_hash>[0-9a-z]{40})\n
    parent[^0-9a-z]+(?P<parent_hash>[0-9a-z]{40})\n
    
    # author
    author[[:blank:]]+
    (?P<author>[a-zA-Z0-9_-]+)
    [[:blank:]]+
    <(?P<author_email>[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[[:alpha:]]{2,})>\n
    .*\n
    \n
    
    # committer
    committer[[:blank:]]+
    (?P<committer>[a-zA-Z0-9_-]+)
    [[:blank:]]+
    <(?P<committer_email>[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[[:alpha:]]{2,})>\n
    .*\n
    \n
    
    # commit message
    (?s)(?P<message>.*)
    "####,
    )
    .unwrap();
    match regex_rule.captures(&commit_string) {
        Some(commit_data) => {
            let extract_data = |name: &str| -> String {
                commit_data
                    .name(name)
                    .map_or("", |data| data.as_str())
                    .to_string()
            };

            let current_hash = extract_data("current_hash");
            let parent_hash = extract_data("parent_hash");
            let author = extract_data("author");
            let author_email = extract_data("author_email");
            let committer = extract_data("committer");
            let committer_email = extract_data("committer_email");
            let message = extract_data("message");

            let author_sign = Signature::new(SignatureType::Author, author, author_email);
            let committer_sign =
                Signature::new(SignatureType::Committer, committer, committer_email);

            Ok(Commit::new(
                author_sign,
                committer_sign,
                string_to_sha(&current_hash)?,
                vec![string_to_sha(&parent_hash)?],
                &message,
            ))
        }
        None => Err(Error::new(
            ErrorKind::InvalidData,
            "Commit data is missing or format is incorrect",
        )),
    }
}

pub async fn push(work_path: &Path, url: &str, index_db: &sled::Db) -> std::io::Result<Response> {
    let new_dbpath = work_path.join("new_tree.db");
    let commitpath = work_path.join("commit");

    println!("PART1");
    // check path is exist
    if !tokio::fs::try_exists(&commitpath).await.unwrap_or(false) {
        eprintln!("Path does not exist: {}", commitpath.display());
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path does not exist: {}", commitpath.display()),
        ));
    }
    // read the file as the body to send
    let commit = extract_commit_from_bytes(&commitpath)?;
    println!("commit id = {}", commit.id._to_string());
    println!("commit tree_id = {}", commit.tree_id._to_string());

    println!("PART2");
    println!("new_dbpath = {}", new_dbpath.display());
    let new_tree_db = sled::open(new_dbpath)?;
    println!("Fin");
    let hashmap = new_tree_db.db_tree_list()?;

    let trees = hashmap.values().cloned().collect::<Vec<Tree>>();
    let blobs = work_path
        .to_path_buf()
        .list_blobs(index_db)
        .unwrap_or(Vec::new());

    let remote_hash = string_to_sha(work_path.file_name().unwrap().to_str().unwrap())?;

    println!("PART3");
    let mut data = BytesMut::new();
    add_pkt_line_string(
        &mut data,
        format!(
            "{} {} {}\0report-status\n",
            remote_hash, commit.id, "refs/heads/main"
        ),
    );
    data.extend_from_slice(b"0000");
    tracing::debug!("{:?}", data);
    data.extend(pack(commit, trees, blobs).await);

    println!("PART4");
    let request = Client::new();

    let url = Url::from_str(url).unwrap();
    let res = request
        .post(url.join("git-receive-pack").unwrap())
        .header(CONTENT_TYPE, "application/x-git-receive-pack-request")
        .body(data.freeze());

    let res = res.send().await.unwrap();

    if res.status() != 200 {
        eprintln!("status code: {}", res.status());
    } else {
        println!("[scorpio]:push seccess!")
    }

    Ok(res)
}
