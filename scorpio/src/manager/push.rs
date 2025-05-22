use bytes::BytesMut;
use ceres::protocol::smart::add_pkt_line_string;
use mercury::internal::object::{ObjectTrait, types::ObjectType};
use reqwest::{Response, header::CONTENT_TYPE, Client, Url};
use std::io::{Error, ErrorKind};
use std::{path::Path, str::FromStr};
use tokio::sync::mpsc;

use crate::manager::store::{BlobFsStore, TreeStore};

use mercury::{
    hash::SHA1,
    internal::{
        object::{blob::Blob, commit::Commit, tree::Tree},
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

pub async fn push(work_path: &Path, url: &str, index_db: &sled::Db) -> std::io::Result<Response> {
    let new_dbpath = work_path.join("new_tree.db");
    let commitpath = work_path.join("commit");

    // check path is exist
    if !tokio::fs::try_exists(&commitpath).await.unwrap_or(false) {
        eprintln!("Path does not exist: {}", commitpath.display());
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path does not exist: {}", commitpath.display()),
        ));
    }
    // read the file as the body to send
    let commit_data = tokio::fs::read(&commitpath).await?;
    let commit_hash = SHA1::from_type_and_data(ObjectType::Commit, &commit_data);

    let new_tree_db = sled::open(new_dbpath)?;
    let hashmap = new_tree_db.db_tree_list()?;

    let trees = hashmap.values().cloned().collect::<Vec<Tree>>();
    let blobs = work_path.to_path_buf().list_blobs(index_db)?;

    let remote_hash = SHA1::from_str(work_path.file_name().unwrap().to_str().unwrap()).unwrap();

    let commit = Commit::from_bytes(&commit_data, commit_hash)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
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

    let request = Client::new();

    let url = Url::from_str(url)
    .unwrap();
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
