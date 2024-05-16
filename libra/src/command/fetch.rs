use std::{collections::HashSet, fs, io::Write, str::FromStr};

use ceres::protocol::ServiceType::UploadPack;
use clap::Parser;
use mercury::{errors::GitError, hash::SHA1};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::io::StreamReader;
use url::Url;

use crate::{
    command::{
        index_pack::{self, IndexPackArgs},
    },
    internal::{
        branch::Branch,
        config::{Config, RemoteConfig},
        head::Head,
        protocol::{https_client::HttpsClient, ProtocolClient},
    },
    utils::{self, path_ext::PathExt},
};
use crate::command::ask_basic_auth;

#[derive(Parser, Debug)]
pub struct FetchArgs {
    #[clap(long, short, group = "sub")]
    repository: Option<String>,

    #[clap(long, short, group = "sub")]
    all: bool,
}

pub async fn execute(args: FetchArgs) {
    tracing::debug!("`fetch` args: {:?}", args);
    tracing::warn!("didn't test yet");
    if args.all {
        let remotes = Config::all_remote_configs().await;
        let tasks = remotes.into_iter().map(|remote| async move {
            fetch_repository(&remote).await;
        });
        futures::future::join_all(tasks).await;
    } else {
        let remote = match args.repository {
            Some(remote) => remote,
            None => "origin".to_string(), // todo: get default remote
        };
        let remote_config = Config::remote_config(&remote).await;
        match remote_config {
            Some(remote_config) => fetch_repository(&remote_config).await,
            None => {
                tracing::error!("remote config '{}' not found", remote);
                eprintln!("fatal: '{}' does not appear to be a git repository", remote);
            }
        }
    }
}

pub async fn fetch_repository(remote_config: &RemoteConfig) {
    println!("fetching from {}", remote_config.name);

    // fetch remote
    let url = match Url::parse(&remote_config.url) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("fatal: invalid URL '{}': {}", remote_config.url, e);
            return;
        }
    };
    let http_client = HttpsClient::from_url(&url);

    let mut refs = http_client.discovery_reference(UploadPack, None).await;
    let mut auth = None;
    while let Err(e) = refs {
        if let GitError::UnAuthorized(_) = e {
            auth = Some(ask_basic_auth());
            refs = http_client
                .discovery_reference(UploadPack, auth.clone())
                .await;
        } else {
            eprintln!("fatal: {}", e);
            return;
        }
    }
    let refs = refs.unwrap();

    let want = refs
        .iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .map(|r| r._hash.clone())
        .collect();
    let have = current_have().await;

    let result_stream = http_client
        .fetch_objects(&have, &want, auth.to_owned())
        .await
        .unwrap();

    let mut reader = StreamReader::new(result_stream);
    let mut line = String::new();

    reader.read_line(&mut line).await.unwrap();
    tracing::info!("First line: {}", line);
    assert_eq!(line, "0008NAK\n");

    /* save pack file */
    let pack_file = {
        // todo how to get total bytes & add progress bar
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            let mut temp_buffer = [0; 4096];
            let n = match reader.read(&mut temp_buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => panic!("error reading from socket; error = {:?}", e),
            };

            buffer.extend_from_slice(&temp_buffer[..n]);
        }

        // todo parse PACK & validate checksum
        let hash = SHA1::new(&buffer[..buffer.len() - 20].to_vec());

        let checksum = SHA1::from_bytes(&buffer[buffer.len() - 20..]);
        assert_eq!(hash, checksum);
        let checksum = checksum.to_plain_str();
        println!("checksum: {}", checksum);

        let pack_file = utils::path::objects()
            .join("pack")
            .join(format!("pack-{}.pack", checksum));
        let mut file = fs::File::create(pack_file.clone()).unwrap();
        file.write_all(&buffer).expect("write failed");

        pack_file.to_string_or_panic()
    };

    /* build .idx file from PACK */
    index_pack::execute(IndexPackArgs {
        pack_file,
        index_file: None,
        index_version: None,
    });

    /* update reference  */
    for reference in refs.iter().filter(|r| r._ref.starts_with("refs/heads")) {
        let branch_name = reference._ref.replace("refs/heads/", "");
        let remote = Some(remote_config.name.as_str());
        Branch::update_branch(&branch_name, &reference._hash, remote).await;
    }
    let remote_head = refs.iter().find(|r| r._ref == "HEAD").unwrap();

    let remote_head_name = refs
        .iter()
        .find(|r| r._ref.starts_with("refs/heads") && r._hash == remote_head._hash);

    match remote_head_name {
        Some(remote_head_name) => {
            let remote_head_name = remote_head_name._ref.replace("refs/heads/", "");
            Head::update(Head::Branch(remote_head_name), Some(&remote_config.name)).await;
        }
        None => {
            // TODO: didn't know how to handle this case
            tracing::error!("remote HEAD points to an unknown branch");
            let hash = SHA1::from_str(&remote_head._hash).unwrap();
            Head::update(Head::Detached(hash), Some(&remote_config.name)).await;
        }
    }
}

async fn current_have() -> Vec<String> {
    // TODO use stand method to generate have list
    let mut have = HashSet::new();
    let branchs = Branch::list_branches(None).await;
    for branch in branchs {
        have.insert(branch.commit.to_plain_str());
    }

    for remote in Config::all_remote_configs().await {
        let branchs = Branch::list_branches(Some(&remote.name)).await;
        for branch in branchs {
            have.insert(branch.commit.to_plain_str());
        }
    }

    have.iter().cloned().collect()
}


