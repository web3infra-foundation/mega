use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

use crate::model::config;
use crate::model::reference::{self, ConfigKind};
use crate::{command, db};
use clap::Parser;
use sea_orm::{ActiveModelTrait, Set};
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
    {
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
        let mut file = fs::File::create(pack_file).unwrap();
        file.write_all(&buffer).expect("write failed");
    }

    /* setup table */
    setup_reference_and_config(refs, remote_repo).await;
}

async fn setup_reference_and_config(refs: Vec<DiscoveredReference>, remote_repo: String) {
    let db = db::get_db_conn().await.unwrap();
    // set remote refes
    let branch_refs: Vec<DiscoveredReference> = refs
        .iter()
        .filter(|r| r._ref.starts_with("refs/heads"))
        .cloned()
        .collect();

    for r in branch_refs.iter() {
        let branch_name = r._ref.replace("refs/heads/", "");
        let origin_branch = reference::ActiveModel {
            name: Set(Some(branch_name)),
            kind: Set(ConfigKind::Branch),
            commit: Set(Some(r._hash.to_owned())),
            remote: Set(Some("origin".to_string())),
            ..Default::default()
        };
        origin_branch.save(&db).await.unwrap();
    }

    let head_ref = refs
        .iter()
        .find(|r| r._ref == "HEAD")
        .expect("orogin HEAD not found");

    // TODO: git may use `refs/heads/branch_name` as branch directly, consider keep it
    let origin_head_name = branch_refs
        .iter()
        .find(|r| r._hash == head_ref._hash)
        .expect("HEAD ref not found in origin refs")
        ._ref
        .replace("refs/heads/", "");

    let origin_head = reference::ActiveModel {
        name: Set(Some(origin_head_name.to_owned())),
        kind: Set(ConfigKind::Head),
        remote: Set(Some("origin".to_string())),
        ..Default::default()
    };
    origin_head.save(&db).await.unwrap();

    // set config: remote.origin.url
    let remote_origin_url = config::ActiveModel {
        configuration: Set("remote".to_owned()),
        name: Set(Some("origin".to_string())),
        key: Set("url".to_owned()),
        value: Set(remote_repo),
        ..Default::default()
    };
    remote_origin_url.save(&db).await.unwrap();

    // set config: remote.origin.fetch
    // todo: temporary ignore fetch option

    // set config: branch.master.remote
    // update HEAD only, because default branch was not created after init
    let mut head: reference::ActiveModel =
        reference::Model::current_head(&db).await.unwrap().into();
    head.name = Set(Some(origin_head_name.to_owned()));
    head.update(&db).await.unwrap();

    let branch_head_remote = config::ActiveModel {
        configuration: Set("branch".to_owned()),
        name: Set(Some(origin_head_name.to_owned())),
        key: Set("remote".to_owned()),
        value: Set("origin".to_owned()),
        ..Default::default()
    };
    branch_head_remote.save(&db).await.unwrap();

    let branch_head_merge = config::ActiveModel {
        configuration: Set("branch".to_owned()),
        name: Set(Some(origin_head_name.to_owned())),
        key: Set("merge".to_owned()),
        value: Set(format!("refs/heads/{}", origin_head_name)),
        ..Default::default()
    };
    branch_head_merge.save(&db).await.unwrap();
}
