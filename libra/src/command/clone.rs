use std::io::Write;

use clap::Parser;
use futures_util::TryStreamExt;
use reqwest::Client;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::io::StreamReader;
use url::Url;

use crate::internal::protocel::https_client::{DiscoveredReference, HttpsClient};
use crate::internal::protocel::ProtocolClient;
use crate::utils::util;

#[derive(Parser, Debug)]
pub struct CloneArgs {
    /// The remote repository location to clone from, usually a URL with HTTPS or SSH
    #[clap(long, short)]
    pub remote_repo: String,

    /// The local path to clone the repository to
    #[clap(long, short)]
    pub local_path: Option<String>,
}

#[allow(unused_variables)] // todo unimplemented
pub async fn execute(args: CloneArgs) {
    let remote_repo = args.remote_repo;
    let local_path = args
        .local_path
        .unwrap_or_else(|| util::cur_dir().to_str().unwrap().to_string());

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

        // 创建一个文件并获取写入器
        let mut file = std::fs::File::create("/tmp/pack").unwrap();

        // 将 StreamReader 包装成 Vec<u8> 以便写入文件
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

        // 将剩余的数据写入文件
        file.write_all(&buffer).expect("write failed");
    }

}
