use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

use crate::command;
use crate::command::index_pack::IndexPackArgs;
use crate::command::restore::RestoreArgs;
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;
use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::io::StreamReader;
use url::Url;
use venus::hash::SHA1;

use crate::internal::protocol::https_client::{DiscoveredReference, HttpsClient};
use crate::internal::protocol::ProtocolClient;
use crate::utils::path_ext::PathExt;
use crate::utils::{path, util};

#[derive(Parser, Debug)]
pub struct CloneArgs {
    /// The remote repository location to clone from, usually a URL with HTTPS or SSH
    pub remote_repo: String,

    /// The local path to clone the repository to
    pub local_path: Option<String>,
}

pub async fn execute(args: CloneArgs) {
    let mut remote_repo = args.remote_repo; // https://gitee.com/caiqihang2024/image-viewer2.0.git
                                            // must end with '/' or Url::join will work incorrectly
    if !remote_repo.ends_with('/') {
        remote_repo.push('/');
    }
    let local_path = args.local_path.unwrap_or_else(|| {
        let repo_name = util::get_repo_name_from_url(&remote_repo).unwrap();
        util::cur_dir().join(repo_name).to_string_or_panic()
    });

    /* create local path */
    let local_path = PathBuf::from(local_path);
    {
        if local_path.exists() && !util::is_empty_dir(&local_path) {
            eprintln!(
                "fatal: destination path '{}' already exists and is not an empty directory.",
                local_path.display()
            );
            return;
        }

        // make sure the directory exists
        if let Err(e) = fs::create_dir_all(&local_path) {
            eprintln!(
                "fatal: could not create directory '{}': {}",
                local_path.display(),
                e
            );
            return;
        }
        let repo_name = local_path.file_name().unwrap().to_str().unwrap();
        println!("Cloning into '{}'", repo_name);
    }

    /* fetch remote */
    let repo_url = Url::parse(&remote_repo).unwrap();
    let client = HttpsClient::from_url(&repo_url);
    let refs = client.discovery_reference().await.unwrap();
    tracing::info!("refs count: {:?}", refs.len());
    tracing::debug!("discovered references: {:?}", refs);

    let want = refs
        .iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .map(|r| r._hash.clone())
        .collect();
    let result_stream = client.fetch_objects(&vec![], &want).await.unwrap();

    let mut reader = StreamReader::new(result_stream);
    let mut line = String::new();

    reader.read_line(&mut line).await.unwrap();
    assert_eq!(line, "0008NAK\n");
    tracing::info!("First line: {}", line);

    // CAUTION: change [current_dir] to the repo directory
    env::set_current_dir(&local_path).unwrap();
    command::init::execute().await;

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

        let pack_file = path::objects()
            .join("pack")
            .join(format!("pack-{}.pack", checksum));
        let mut file = fs::File::create(pack_file.clone()).unwrap();
        file.write_all(&buffer).expect("write failed");

        pack_file.to_string_or_panic()
    };

    // build .idx file from PACK
    command::index_pack::execute(IndexPackArgs {
        pack_file,
        index_file: None,
        index_version: None,
    });

    /* setup table */
    setup_reference_and_config(refs, remote_repo).await;

    // restore all files to worktree from HEAD
    command::restore::execute(RestoreArgs {
        worktree: true,
        staged: true,
        source: None,
        pathspec: vec![util::working_dir_string()],
    })
    .await;
}

async fn setup_reference_and_config(refs: Vec<DiscoveredReference>, remote_repo: String) {
    const ORIGIN: &str = "origin"; // default remote name, prevent spelling mistakes

    let branch_refs: Vec<DiscoveredReference> = refs
        .iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .cloned()
        .collect();

    // set remote refs
    for r in branch_refs.iter() {
        let branch_name = r._ref.replace("refs/heads/", "");
        Branch::update_branch(&branch_name, &r._hash, Some(ORIGIN)).await;
    }

    let head_ref = refs
        .iter()
        .find(|r| r._ref == "HEAD")
        .expect("origin HEAD not found");

    // TODO: git may use `refs/heads/branch_name` as branch directly, consider keep it
    let origin_head = branch_refs
        .iter()
        .find(|r| r._hash == head_ref._hash)
        .expect("HEAD ref not found in origin refs")
        ._ref
        .clone();

    let origin_head_name = origin_head.replace("refs/heads/", "");

    {
        let _origin_head = Head::Branch(origin_head_name.clone());
        // update remote HEAD, default `origin`
        Head::update(_origin_head.to_owned(), Some(ORIGIN)).await;
        // update HEAD only, because default branch was not created after init
        Head::update(_origin_head, None).await; // local HEAD
    }
    // set config: remote.origin.url
    Config::insert("remote", Some(ORIGIN), "url", &remote_repo).await;

    // set config: remote.origin.fetch
    // todo: temporary ignore fetch option

    // set config: branch.master.remote
    Config::insert("branch", Some(&origin_head_name), "remote", ORIGIN).await;
    // set config: branch.master.merge
    Config::insert("branch", Some(&origin_head_name), "merge", &origin_head).await;
}
