use ceres::protocol::ServiceType::{self, UploadPack};
use clap::Parser;
use indicatif::ProgressBar;
use mercury::hash::SHA1;
use mercury::internal::object::commit::Commit;
use sea_orm::TransactionTrait;
use std::io;
use std::time::Instant;
use std::vec;
use std::{collections::HashSet, fs, io::Write};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::io::StreamReader;

use crate::command::load_object;
use crate::internal::db::get_db_conn_instance;
use crate::internal::reflog::{HEAD, Reflog, ReflogAction, ReflogContext, ReflogError, zero_sha1};
use crate::utils::util;
use crate::{
    command::index_pack::{self, IndexPackArgs},
    internal::{
        branch::Branch,
        config::{Config, RemoteConfig},
        head::Head,
        protocol::{
            DiscRef, FetchStream, ProtocolClient, https_client::HttpsClient,
            local_client::LocalClient,
        },
    },
    utils::{self, path_ext::PathExt},
};
use mercury::errors::GitError;
use std::io::Error as IoError;
use url::Url;

const DEFAULT_REMOTE: &str = "origin";

enum RemoteClient {
    Http(HttpsClient),
    Local(LocalClient),
}

impl RemoteClient {
    fn from_spec(spec: &str) -> Result<Self, String> {
        if let Ok(url) = Url::parse(spec) {
            match url.scheme() {
                "http" | "https" => Ok(Self::Http(HttpsClient::from_url(&url))),
                "file" => {
                    let path = url
                        .to_file_path()
                        .map_err(|_| format!("invalid file url: {spec}"))?;
                    let client = LocalClient::from_path(path)
                        .map_err(|e| format!("invalid local repository '{}': {}", spec, e))?;
                    Ok(Self::Local(client))
                }
                other => Err(format!("unsupported remote scheme '{other}'")),
            }
        } else {
            if spec.contains("://") || spec.contains('@') {
                return Err(format!(
                    "unsupported remote specification '{spec}': protocol not implemented"
                ));
            }
            let normalized = spec.trim_end_matches('/');
            let client = LocalClient::from_path(normalized)
                .map_err(|e| format!("invalid local repository '{}': {}", spec, e))?;
            Ok(Self::Local(client))
        }
    }

    async fn discovery_reference(&self, service: ServiceType) -> Result<Vec<DiscRef>, GitError> {
        match self {
            RemoteClient::Http(client) => client.discovery_reference(service).await,
            RemoteClient::Local(client) => client.discovery_reference(service).await,
        }
    }

    async fn fetch_objects(
        &self,
        have: &[String],
        want: &[String],
    ) -> Result<FetchStream, IoError> {
        match self {
            RemoteClient::Http(client) => client.fetch_objects(have, want).await,
            RemoteClient::Local(client) => client.fetch_objects(have, want).await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct FetchArgs {
    /// Repository to fetch from
    pub repository: Option<String>,

    /// Refspec to fetch, usually a branch name
    #[clap(requires("repository"))]
    pub refspec: Option<String>,

    /// Fetch all remotes.
    #[clap(long, short, conflicts_with("repository"))]
    pub all: bool,
}

pub async fn execute(args: FetchArgs) {
    tracing::debug!("`fetch` args: {:?}", args);
    tracing::warn!("didn't test yet");
    if args.all {
        let remotes = Config::all_remote_configs().await;
        let tasks = remotes.into_iter().map(|remote| async move {
            fetch_repository(remote, None).await;
        });
        futures::future::join_all(tasks).await;
    } else {
        let remote = match args.repository {
            Some(remote) => remote,
            None => Config::get_current_remote()
                .await
                .unwrap_or_else(|_| {
                    eprintln!("fatal: HEAD is detached");
                    Some(DEFAULT_REMOTE.to_owned())
                })
                .unwrap_or_else(|| {
                    eprintln!("fatal: No remote configured for current branch");
                    DEFAULT_REMOTE.to_owned()
                }),
        };
        let remote_config = Config::remote_config(&remote).await;
        match remote_config {
            Some(remote_config) => fetch_repository(remote_config, args.refspec).await,
            None => {
                tracing::error!("remote config '{}' not found", remote);
                eprintln!("fatal: '{remote}' does not appear to be a libra repository");
            }
        }
    }
}

/// Fetch from remote repository
/// - `branch` is optional, if `None`, fetch all branches
pub async fn fetch_repository(remote_config: RemoteConfig, branch: Option<String>) {
    println!(
        "fetching from {}{}",
        remote_config.name,
        if let Some(branch) = &branch {
            format!(" ({branch})")
        } else {
            "".to_owned()
        }
    );

    // fetch remote
    let remote_client = match RemoteClient::from_spec(&remote_config.url) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };

    let mut refs = match remote_client.discovery_reference(UploadPack).await {
        Ok(refs) => refs,
        Err(e) => {
            eprintln!("fatal: {e}");
            return;
        }
    };

    if refs.is_empty() {
        tracing::warn!("fetch empty, no refs found");
        return;
    }

    let remote_head = refs.iter().find(|r| r._ref == HEAD).cloned();
    // remote branches
    let ref_heads = refs
        .clone()
        .into_iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .collect::<Vec<_>>();

    // filter by branch
    if let Some(ref branch) = branch {
        let branch = if !branch.starts_with("refs") {
            format!("refs/heads/{branch}")
        } else {
            branch.to_owned()
        };
        refs.retain(|r| r._ref == branch);

        if refs.is_empty() {
            eprintln!("fatal: '{branch}' not found in remote");
            return;
        }
    }

    let want = refs.iter().map(|r| r._hash.clone()).collect::<Vec<_>>();
    let have = current_have().await; // TODO: return `DiscRef` rather than only hash, to compare `have` & `want` more accurately
    let mut result_stream = match remote_client.fetch_objects(&have, &want).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("fatal: failed to fetch objects: {e}");
            return;
        }
    };

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
        if reach_pack {
            // 2.PACK data
            let bytes_per_sec = pack_data.len() as f64 / time.elapsed().as_secs_f64();
            let total = util::auto_unit_bytes(pack_data.len() as u64);
            let bps = util::auto_unit_bytes(bytes_per_sec as u64);
            bar.set_message(format!("Receiving objects: {total:.2} | {bps:.2}/s"));
            bar.tick();
            // Side-Band Capability, should be enabled if Server Support
            let code = data[0];
            let data = &data[1..];
            match code {
                1 => {
                    // Data
                    pack_data.extend(data); // TODO: decode meanwhile & calc progress
                }
                2 => {
                    // Progress
                    print!("{}", String::from_utf8_lossy(data));
                    io::stdout().flush().unwrap();
                }
                3 => {
                    // Error
                    eprintln!("{}", String::from_utf8_lossy(data));
                }
                _ => {
                    eprintln!("unknown side-band-64k code: {code}");
                }
            }
        } else if &data != b"NAK\n" {
            // 1.front info (server progress), ignore NAK (first line)
            print!("{}", String::from_utf8_lossy(&data)); // data contains '\r' & '\n' at end
            io::stdout().flush().unwrap();
        }
    }
    bar.finish();

    /* save pack file */
    let pack_file = if pack_data.len() < 20 {
        tracing::debug!("No pack data returned from remote");
        None
    } else {
        let payload_len = pack_data.len() - 20;
        let hash = SHA1::new(&pack_data[..payload_len]);

        let checksum = SHA1::from_bytes(&pack_data[payload_len..]);
        assert_eq!(hash, checksum);
        let checksum = checksum.to_string();
        println!("checksum: {checksum}");

        if pack_data.len() > 32 {
            // 12 header + 20 hash
            let pack_file = utils::path::objects()
                .join("pack")
                .join(format!("pack-{checksum}.pack"));
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

    let db = get_db_conn_instance().await;
    let transaction_result = db
        .transaction(|txn| {
            Box::pin(async move {
                // 1. Update remote-tracking branches and record reflogs
                for r in &refs {
                    let full_ref_name: String;

                    // Determine the full ref name (e.g., "refs/remotes/origin/main")
                    if let Some(branch_name) = r._ref.strip_prefix("refs/heads/") {
                        full_ref_name =
                            format!("refs/remotes/{}/{}", remote_config.name, branch_name);
                    } else if let Some(cl_name) = r._ref.strip_prefix("refs/cl/") {
                        // Handle change lists if your system supports them
                        full_ref_name =
                            format!("refs/remotes/{}/cl/{}", remote_config.name, cl_name);
                    } else {
                        tracing::warn!("Unsupported ref type during fetch: {}", r._ref);
                        continue; // Skip unsupported ref types
                    }

                    // Get the old OID *before* updating the branch
                    let old_oid = Branch::find_branch_with_conn(txn, &full_ref_name, None)
                        .await
                        .map_or(zero_sha1().to_string(), |b| b.commit.to_string());

                    // Update the branch pointer
                    Branch::update_branch_with_conn(txn, &full_ref_name, &r._hash, None).await;

                    // Prepare and insert the reflog entry for this specific remote-tracking branch
                    let context = ReflogContext {
                        old_oid: old_oid.to_string(),
                        new_oid: r._hash.clone(),
                        action: ReflogAction::Fetch, // Using a simple Fetch action
                    };
                    Reflog::insert_single_entry(txn, &context, &full_ref_name).await?;
                }

                // 2. Update the remote's HEAD pointer
                if let Some(remote_head) = remote_head {
                    if let Some(remote_head_ref) =
                        ref_heads.iter().find(|r| r._hash == remote_head._hash)
                    {
                        if let Some(remote_head_branch) =
                            remote_head_ref._ref.strip_prefix("refs/heads/")
                        {
                            // This updates `refs/remotes/origin/HEAD`
                            Head::update_with_conn(
                                txn,
                                Head::Branch(remote_head_branch.to_owned()),
                                Some(&remote_config.name),
                            )
                            .await;
                        }
                    } else if branch.is_none() {
                        eprintln!("remote HEAD not found");
                    } else {
                        tracing::debug!("Specified branch not found in remote HEAD");
                    }
                } else {
                    tracing::warn!("fetch empty, remote HEAD not found");
                }
                Ok::<_, ReflogError>(())
            })
        })
        .await;

    if let Err(e) = transaction_result {
        eprintln!("fatal: failed to update references after fetch: {}", e);
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
            if inserted.contains(&commit.id.to_string()) {
                return;
            }
            inserted.insert(commit.id.to_string());
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
        have.push(item.commit.to_string());

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
