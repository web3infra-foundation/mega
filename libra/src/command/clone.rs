use std::{env, fs};
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::io::StreamReader;
use url::Url;
use venus::hash::SHA1;
use crate::command;

use crate::internal::protocol::https_client::HttpsClient;
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

    let local_path = PathBuf::from(local_path);
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

    let repo_url = Url::parse(&remote_repo).unwrap();
    let client = HttpsClient::from_url(&repo_url);
    let refs = client.discovery_reference().await.unwrap();
    tracing::info!("refs count: {:?}", refs.len());
    tracing::debug!("discovered references: {:?}", refs);

    let want = refs
        .iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .map(|r| r.hash.clone())
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

    // todo consider unpacking the pack file directly

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

    let pack_file = path::objects().join("pack").join(format!("pack-{}.pack", checksum));
    let mut file = fs::File::create(pack_file).unwrap();
    file.write_all(&buffer).expect("write failed");
}
