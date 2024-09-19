use std::io;
use std::vec;
use std::{collections::HashSet, fs, io::Write};
use std::time::Instant;
use ceres::protocol::ServiceType::UploadPack;
use clap::Parser;
use indicatif::ProgressBar;
use mercury::internal::object::commit::Commit;
use mercury::{errors::GitError, hash::SHA1};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::io::StreamReader;
use url::Url;

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
use crate::utils::util;

const DEFAULT_REMOTE: &str = "origin";

#[derive(Parser, Debug)]
pub struct FetchArgs {
    #[clap(group = "sub")]
    pub repository: Option<String>,

    // TODO: refspec

    #[clap(long, short, group = "sub")]
    pub all: bool,
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
            None => Config::get_current_remote().await.unwrap_or_else(|_| {
                eprintln!("fatal: HEAD is detached");
                Some(DEFAULT_REMOTE.to_owned())
            }).unwrap_or_else(|| {
                eprintln!("fatal: No remote configured for current branch");
                DEFAULT_REMOTE.to_owned()
            }),
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
        .collect::<Vec<_>>();
    let have = current_have().await; // TODO: return `DiscRef` rather than only hash, to compare `have` & `want` more accurately

    let mut result_stream = http_client
        .fetch_objects(&have, &want, auth.to_owned())
        .await
        .unwrap();

    let mut reader = StreamReader::new(&mut result_stream);
    let mut pack_data = Vec::new();
    let mut reach_pack = false;
    let bar = ProgressBar::new_spinner();
    let time = Instant::now();
    loop {
        let (len, data) = read_pkt_line(&mut reader).await.unwrap();
        if len == 0 {
            break;
        }
        if data.len() >= 5 && &data[1..5] == b"PACK" {
            reach_pack = true;
            tracing::debug!("Receiving PACK data...");
        }
        if reach_pack { // 2.PACK data
            let bytes_per_sec = pack_data.len() as f64 / time.elapsed().as_secs_f64();
            let total = util::auto_unit_bytes(pack_data.len() as u64);
            let bps = util::auto_unit_bytes(bytes_per_sec as u64);
            bar.set_message(format!("Receiving objects: {total:.2} | {bps:.2}/s"));
            bar.tick();
            // Side-Band Capability, should be enabled if Server Support
            let code = data[0];
            let data = &data[1..];
            match code {
                1 => { // Data
                    pack_data.extend(data); // TODO: decode meanwhile & calc progress
                }
                2 => { // Progress
                    print!("{}", String::from_utf8_lossy(data));
                    std::io::stdout().flush().unwrap();
                }
                3 => { // Error
                    eprintln!("{}", String::from_utf8_lossy(data));
                }
                _ => {
                    eprintln!("unknown side-band-64k code: {}", code);
                }
            }
        } else if &data != b"NAK\n" { // 1.front info (server progress), ignore NAK (first line)
            print!("{}", String::from_utf8_lossy(&data)); // data contains '\r' & '\n' at end
            std::io::stdout().flush().unwrap();
        }
    };
    bar.finish();

    /* save pack file */
    let pack_file = {
        let hash = SHA1::new(&pack_data[..pack_data.len() - 20].to_vec());

        let checksum = SHA1::from_bytes(&pack_data[pack_data.len() - 20..]);
        assert_eq!(hash, checksum);
        let checksum = checksum.to_plain_str();
        println!("checksum: {}", checksum);

        if pack_data.len() > 32 { // 12 header + 20 hash
            let pack_file = utils::path::objects()
                .join("pack")
                .join(format!("pack-{}.pack", checksum));
            let mut file = fs::File::create(pack_file.clone()).unwrap();
            file.write_all(&pack_data).expect("write failed");

            Some(pack_file.to_string_or_panic())
        } else {
            tracing::debug!("Empty pack file");
            None
        }
    };

    if let Some(pack_file) = pack_file {
        /* build .idx file from PACK */
        index_pack::execute(IndexPackArgs {
            pack_file,
            index_file: None,
            index_version: None,
        });
    }

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

/// Read 4 bytes hex number
async fn read_hex_4(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf).await?;
    let hex_str = std::str::from_utf8(&buf).unwrap();
    u32::from_str_radix(hex_str, 16).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
/// async version of `read_pkt_line`
/// - return (raw length, data)
async fn read_pkt_line(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<(usize, Vec<u8>)> {
    let len = read_hex_4(reader).await?;
    if len == 0 {
        return Ok((0, Vec::new()));
    }
    let mut data = vec![0u8; (len - 4) as usize];
    reader.read_exact(&mut data).await?;
    Ok((len as usize, data))
}