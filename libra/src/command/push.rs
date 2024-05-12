use clap::Parser;
use crate::internal::branch::Branch;
use crate::internal::config::Config;
use crate::internal::head::Head;

#[derive(Parser, Debug)]
pub struct PushArgs {
    /// repository
    repository: Option<String>,
    /// ref to push
    refspec: Option<String>,
}

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

    let refspec = args.refspec.unwrap_or(branch);
    let commit_hash = Branch::find_branch(&refspec, None).await.unwrap().commit;

    println!("pushing to {} {}({})", repository, refspec, commit_hash);
}