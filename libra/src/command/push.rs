use clap::Parser;
use url::Url;
use ceres::protocol::ServiceType::ReceivePack;
use venus::errors::GitError;
use crate::command::ask_username_password;
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;
use crate::internal::protocol::https_client::HttpsClient;
use crate::internal::protocol::ProtocolClient;

#[derive(Parser, Debug)]
pub struct PushArgs {
    /// repository, e.g. origin
    repository: Option<String>,
    /// ref to push, e.g. master
    refspec: Option<String>,
}

#[allow(unused_variables)]
pub async fn execute(args: PushArgs) {
    let branch = match Head::current().await {
        Head::Branch(name) => name,
        Head::Detached(_) => panic!("fatal: HEAD is detached while pushing"),
    };

    let repository = match args.repository {
        Some(repo) => repo,
        None => {
            // e.g. [branch "master"].remote = origin
            Config::get("branch", Some(&branch), "remote").await.unwrap()
        }
    };
    let repo_url = Config::get("remote", Some(&repository), "url").await.unwrap();

    let refspec = args.refspec.unwrap_or(branch);
    let commit_hash = Branch::find_branch(&refspec, None).await.unwrap().commit;

    println!("pushing {}({}) to {}({})", refspec, commit_hash, repository, repo_url);

    let url = Url::parse(&repo_url).unwrap();
    let client = HttpsClient::from_url(&url);
    let refs = client.discovery_reference(ReceivePack, None).await;
    let refs = match refs {
        Ok(refs) => refs,
        Err(e) => {
            if let GitError::UnAuthorized(e) = e {
                eprintln!("fatal: {}", e);
                let (username, password) = ask_username_password();
                client.discovery_reference(ReceivePack, Some((username, Some(password)))).await.unwrap()
            } else {
                eprintln!("fatal: {}", e);
                return;
            }
        }
    };
    // TODO
}