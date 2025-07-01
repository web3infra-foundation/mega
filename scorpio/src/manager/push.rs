use bytes::BytesMut;
use ceres::protocol::smart::add_pkt_line_string;
use chrono::DateTime;
use regex::Regex;
use reqwest::{header::CONTENT_TYPE, ClientBuilder, Response, Url};
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

/// Use Git style to package Commit, Tree objects, and
/// Blobs objects
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

/// Convert a String to a SHA1 hash
fn string_to_sha(hash: &str) -> std::io::Result<SHA1> {
    SHA1::from_str(hash).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

/// Extract commit information from a commit file
fn extract_commit_from_bytes(commitpath: &Path) -> std::io::Result<Commit> {
    let commit_string = std::fs::read_to_string(commitpath)?;
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
    Date: (?P<author_time>.*)\n
    \n

    # committer
    committer[[:blank:]]+
    (?P<committer>[a-zA-Z0-9_-]+)
    [[:blank:]]+
    <(?P<committer_email>[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[[:alpha:]]{2,})>\n
    Date: (?P<commit_time>.*)\n
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
            let author_time = extract_data("author_time");
            let committer = extract_data("committer");
            let committer_email = extract_data("committer_email");
            let commit_time = extract_data("commit_time");
            let message = extract_data("message");

            println!("author_time = {author_time}");
            println!("commit_time = {commit_time}");

            // This part of the code is to prevent the timestamp
            // change from causing the Commit Hash to change
            let author_time = DateTime::parse_from_str(&author_time, "%Y-%m-%d %H:%M:%S UTC %z")
                .map_err(|e| Error::new(ErrorKind::InvalidData, e))?
                .to_utc();
            let commit_time = DateTime::parse_from_str(&commit_time, "%Y-%m-%d %H:%M:%S UTC %z")
                .map_err(|e| Error::new(ErrorKind::InvalidData, e))?
                .to_utc();
            /*
            println!("author_time = {:?}", author_time);
            println!("commit_time = {:?}", commit_time);
            println!("now = {:?}", chrono::Utc::now());
            println!("now = {}", chrono::Utc::now());
            */
            let author_sign = Signature::from_data(
                format!(
                    "{} {} <{}> {} +0800",
                    SignatureType::Author,
                    author,
                    author_email,
                    author_time.timestamp()
                )
                .to_string()
                .into_bytes(),
            )
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
            let committer_sign = Signature::from_data(
                format!(
                    "{} {} <{}> {} +0800",
                    SignatureType::Author,
                    committer,
                    committer_email,
                    commit_time.to_utc().timestamp()
                )
                .to_string()
                .into_bytes(),
            )
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

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

/// Push the commit to the remote repository
pub async fn push_core(
    work_path: &Path,
    url: &str,
    index_db: &sled::Db,
) -> std::io::Result<Response> {
    let new_dbpath = work_path.join("new_tree.db");
    let commitpath = work_path.join("commit");

    println!("\x1b[34m[PART1]\x1b[0m");
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
    /*
    println!("commit id = {}", commit.id._to_string());
    println!("commit tree_id = {}", commit.tree_id._to_string());
    println!("commit = {:?}", commit);
    */

    println!("\x1b[34m[PART2]\x1b[0m");
    // println!("new_dbpath = {}", new_dbpath.display());
    let new_tree_db = sled::open(new_dbpath)?;
    println!("Fin");
    let hashmap = new_tree_db.db_tree_list()?;

    let trees = hashmap.values().cloned().collect::<Vec<Tree>>();
    let blobs = work_path
        .to_path_buf()
        .list_blobs(index_db)
        .unwrap_or_default();

    let remote_hash = string_to_sha(work_path.file_name().unwrap().to_str().unwrap())?;

    println!("\x1b[34m[PART3]\x1b[0m");
    let mut data = BytesMut::new();
    add_pkt_line_string(
        &mut data,
        format!(
            "{} {} {}\0report-status\n",
            remote_hash, commit.id, "refs/heads/main"
        ),
    );
    data.extend_from_slice(b"0000");

    // tracing::debug!("{:?}", data);
    data.extend(pack(commit, trees, blobs).await);

    println!("\x1b[34m[PART4]\x1b[0m");
    let request = ClientBuilder::new().build().unwrap();

    // println!("data = {:?}", data.clone().freeze());
    // println!("url = {url}");
    let url = Url::from_str(url).unwrap();

    let res = request
        .post(url)
        .header(CONTENT_TYPE, "application/x-git-receive-pack-request")
        .body(data.freeze());

    println!("send_pack request: {res:?}");

    let res = match res.send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("\x1b[31mFailed to send request: {e:?}\x1b[0m");
            return Err(std::io::Error::other("Failed to send request"));
        }
    };

    if res.status() != 200 {
        eprintln!("status code: {}", res.status());
    } else {
        println!("[scorpio]:\x1b[32mpush seccess!\x1b[0m");
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_extract_commit_from_bytes() {
        let tmp_path = PathBuf::from("/tmp/tmp13384");
        let commit_data = r###"tree: 73a486aae34e5c2afe7bc164a54569b98133af4d
parent: 54f28ffd6c8aece72eb9138bfaa44ad0dacfb2ff
author MEGA <admin@mega.org>
Date: 2025-06-06 03:33:05 UTC +0800

committer MEGA <admin@mega.org>
Date: 2025-06-06 03:33:05 UTC +0800

Added some tmp files"###;
        std::fs::create_dir_all(&tmp_path).expect("Failed to create tmp directory");
        let commit_path = tmp_path.join("commit");
        std::fs::write(&commit_path, commit_data).expect("Failed to write commit data");

        match extract_commit_from_bytes(&commit_path) {
            Ok(commit) => {
                std::fs::remove_dir_all(&tmp_path).expect("Failed to remove tmp directory");
                assert_eq!(
                    &commit.id._to_string(),
                    "1c85c8908b8eb0e4bc62ff834cae3038ca2930f5"
                );
                assert_eq!(
                    &commit.tree_id._to_string(),
                    "73a486aae34e5c2afe7bc164a54569b98133af4d"
                );
                assert_eq!(
                    &commit.parent_commit_ids[0]._to_string(),
                    "54f28ffd6c8aece72eb9138bfaa44ad0dacfb2ff"
                );
                println!("\x1b[32mParse successful!\x1b[0m");
            }
            Err(e) => {
                std::fs::remove_dir_all(&tmp_path).expect("Failed to remove tmp directory");
                eprintln!("\x1b[31mParse failed: {e}\x1b[0m");
            }
        }
    }
}
