use std::io::Read;
use std::{collections::HashSet, fs, io::Write};

use ceres::protocol::ServiceType::UploadPack;
use clap::Parser;
use futures::StreamExt;
use mercury::internal::object::commit::Commit;
use mercury::utils::read_bytes;
use mercury::{errors::GitError, hash::SHA1};
use url::Url;
use byteorder::ReadBytesExt;

use crate::command::{ask_basic_auth, load_object};
use crate::{
    command::index_pack::{self, IndexPackArgs},
    internal::{
        branch::Branch,
        config::{Config, RemoteConfig},
        head::Head,
        protocol::{https_client::HttpsClient, ProtocolClient},
    },
    utils::{self, path_ext::PathExt},
};

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
    if refs.is_empty() {
        tracing::warn!("fetch empty, no refs found");
        return;
    }

    let want = refs
        .iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .map(|r| r._hash.clone())
        .collect();
    let have = current_have().await;

    let mut result_stream = http_client
        .fetch_objects(&have, &want, auth.to_owned())
        .await
        .unwrap();

    let mut buffer = vec![];
    while let Some(item) = result_stream.next().await {
        let item = item.unwrap();
        buffer.extend(item);
    }

    let mut pack_data = Vec::new();
    // pase pkt line
    if let Some(pack_pos) = buffer.windows(4).position(|w| w == b"PACK") {
        tracing::info!("pack data found at: {}", pack_pos);
        let readable_output = std::str::from_utf8(&buffer[..pack_pos]).unwrap();
        tracing::debug!("stdout readable: \n{}", readable_output);
        tracing::info!("pack length: {}", buffer.len() - pack_pos);
        assert!(buffer[pack_pos..pack_pos + 4].eq(b"PACK"));

        let side_band = true; // TODO: Check Client & Server Capability
        if !side_band {
            pack_data = buffer[pack_pos..].to_vec(); // no side-band-64k
        } else {
            buffer = buffer[pack_pos - 5..].to_vec();
            // buffer -> stream
            let mut reader = std::io::Cursor::new(&buffer);
            loop {
                let len_bytes = read_bytes(&mut reader, 4).unwrap();
                let len_str = std::str::from_utf8(&len_bytes).unwrap();
                let mut len = u32::from_str_radix(len_str, 16).unwrap();
                if len == 0 {
                    tracing::info!("pack data read done");
                    break;
                }
                let code = reader.read_u8().unwrap();
                len -= 5; // 1 byte for code, 4 bytes for length
                let mut data = vec![0u8; len as usize];
                reader.read_exact(&mut data).unwrap();

                match code {
                    1 => { // Data
                        pack_data.extend(data);
                    }
                    2 => { // Progress
                        tracing::info!("progress: {}", std::str::from_utf8(&data).unwrap());
                    }
                    3 => { // Error
                        tracing::error!("stderr: {}", std::str::from_utf8(&data).unwrap());
                    }
                    _ => {
                        tracing::warn!("unknown side-band-64k code: {}", code);
                    }
                }
            }
        }
    } else {
        tracing::error!(
            "no pack data found, stdout is: \n{}",
            std::str::from_utf8(&buffer).unwrap()
        );
        panic!("no pack data found");
    }

    /* save pack file */
    let pack_file = {
        let hash = SHA1::new(&pack_data[..pack_data.len() - 20].to_vec());

        let checksum = SHA1::from_bytes(&pack_data[pack_data.len() - 20..]);
        assert_eq!(hash, checksum);
        let checksum = checksum.to_plain_str();
        println!("checksum: {}", checksum);

        let pack_file = utils::path::objects()
            .join("pack")
            .join(format!("pack-{}.pack", checksum));
        let mut file = fs::File::create(pack_file.clone()).unwrap();
        file.write_all(&pack_data).expect("write failed");

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
    let remote_head = refs.iter().find(|r| r._ref == "HEAD");
    match remote_head {
        Some(remote_head) => {
            let remote_head_name = refs
                .iter()
                .find(|r| r._ref.starts_with("refs/heads") && r._hash == remote_head._hash);

            match remote_head_name {
                Some(remote_head_name) => {
                    let remote_head_name = remote_head_name._ref.replace("refs/heads/", "");
                    Head::update(Head::Branch(remote_head_name), Some(&remote_config.name)).await;
                }
                None => {
                    panic!("remote HEAD not found")
                }
            }
        }
        None => {
            tracing::warn!("fetch empty, remote HEAD not found");
        }
    }
}

async fn current_have() -> Vec<String> {
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    struct QueueItem {
        priority: usize,
        commit: SHA1,
    }
    let mut c_pending = std::collections::BinaryHeap::new();
    let mut inserted = HashSet::new();
    let check_and_insert =
        |commit: &Commit,
         inserted: &mut HashSet<String>,
         c_pending: &mut std::collections::BinaryHeap<QueueItem>| {
            if inserted.contains(&commit.id.to_plain_str()) {
                return;
            }
            inserted.insert(commit.id.to_plain_str());
            c_pending.push(QueueItem {
                priority: commit.committer.timestamp,
                commit: commit.id,
            });
        };
    let mut remotes = Config::all_remote_configs()
        .await
        .iter()
        .map(|r| Some(r.name.to_owned()))
        .collect::<Vec<_>>();
    remotes.push(None);

    for remote in remotes {
        let branchs = Branch::list_branches(remote.as_deref()).await;
        for branch in branchs {
            let commit: Commit = load_object(&branch.commit).unwrap();
            check_and_insert(&commit, &mut inserted, &mut c_pending);
        }
    }
    let mut have = Vec::new();
    while have.len() < 32 && !c_pending.is_empty() {
        let item = c_pending.pop().unwrap();
        have.push(item.commit.to_plain_str());

        let commit: Commit = load_object(&item.commit).unwrap();
        for parent in commit.parent_commit_ids {
            let parent: Commit = load_object(&parent).unwrap();
            check_and_insert(&parent, &mut inserted, &mut c_pending);
        }
    }

    have
}
