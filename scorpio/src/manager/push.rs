use bytes::BytesMut;
use ceres::protocol::smart::add_pkt_line_string;
use reqwest::{header::CONTENT_TYPE, Client, Url};
use std::{path::PathBuf, str::FromStr};
use tokio::sync::mpsc;

use crate::manager::diff::change;
use mercury::{
    hash::SHA1,
    internal::{
        object::{blob::Blob, commit::Commit, signature::Signature, tree::Tree},
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
#[allow(unused)]
pub async fn push(path: PathBuf, monopath: PathBuf) -> std::io::Result<()> {
    let mut lower = path.clone();
    lower.push("lower");
    let mut upper = path.clone();
    upper.push("upper");
    let mut dbpath = path.clone();
    dbpath.push("tree.db");

    let db = sled::open(dbpath)?;
    let mut trees = Vec::new();
    let mut blobs = Vec::new();
    let root_tree = change(upper, monopath.clone(), &mut trees, &mut blobs, &db);
    trees.push(root_tree.clone());
    let default_author = Signature::from_data(
        "author Quanyi Ma <eli@patch.sh> 1678101573 +0800"
            .to_string()
            .into_bytes(),
    )
    .unwrap();

    let remote_hash = SHA1::from_str(path.file_name().unwrap().to_str().unwrap()).unwrap();

    let commit = Commit::new(
        default_author.clone(),
        default_author,
        root_tree.id,
        vec![remote_hash],
        "test commit ",
    );
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

    let url = Url::from_str(&format!(
        "http://localhost:8000/{}",
        monopath.to_str().unwrap()
    ))
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

    Ok(())
}
