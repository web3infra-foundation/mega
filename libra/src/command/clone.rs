use std::fs;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use futures_util::TryStreamExt;
use reqwest::Client;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::io::StreamReader;
use url::Url;

use crate::internal::protocel::https_client::{DiscoveredReference, HttpsClient};
use crate::internal::protocel::ProtocolClient;
use crate::utils::path_ext::PathExt;
use crate::utils::util;

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
    let local_path = args
        .local_path
        .unwrap_or_else(|| {
            let repo_name = util::get_repo_name_from_url(&remote_repo).unwrap();
            util::cur_dir().join(repo_name).to_string_or_panic()
        });

    let local_path = PathBuf::from(local_path);
    if local_path.exists() && !util::is_empty_dir(&local_path) {
        eprintln!("fatal: destination path '{}' already exists and is not an empty directory.", local_path.display());
        return;
    }

    // make sure the directory exists
    if let Err(e) = fs::create_dir_all(&local_path) {
        eprintln!("fatal: could not create directory '{}': {}", local_path.display(), e);
        return;
    }
    let repo_name = local_path.file_name().unwrap().to_str().unwrap();
    println!("Cloning into '{}'", repo_name);

    let repo_url = Url::parse(&remote_repo).unwrap();
    let url = repo_url.join("git-upload-pack").unwrap();
    let client = HttpsClient::from_url(&repo_url);
    let refs = client.discovery_reference().await.unwrap();
    let refs: Vec<DiscoveredReference> = refs
        .iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .cloned()
        .collect();
    println!("{:?}", refs);

    let client = Client::builder().http1_only().build().unwrap();
    let mut body = String::new();
    for r in refs {
        body += format!("0032want {}\n", r.hash).as_str();
    }
    body += "00000009done\n"; // '\n' is important or no response!
    println!("body:\n{}\n", body);
    let res = client
        .post(url)
        .header("Content-Type", "application/x-git-upload-pack-request")
        .body(body)
        .send()
        .await
        .unwrap();
    println!("{:?}", res.status());

    if res.status().is_success() {
        let stream = res.bytes_stream().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
        let mut reader = StreamReader::new(stream);
        let mut line = String::new();

        reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "0008NAK\n");
        println!("First line: {}", line);

        let mut file = fs::File::create(local_path.join("tempPACK.pack")).unwrap();

        let mut buffer: Vec<u8> = Vec::new();
        loop {
            let mut temp_buffer = [0; 1024];
            let n = match reader.read(&mut temp_buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => panic!("error reading from socket; error = {:?}", e)
            };

            buffer.extend_from_slice(&temp_buffer[..n]);
        }

        file.write_all(&buffer).expect("write failed");
    }

}
