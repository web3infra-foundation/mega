use std::collections::HashSet;

use clap::Parser;

use crate::internal::config::Config;

#[derive(Parser, Debug)]
pub struct FetchArgs {
    #[clap(long, short, group = "sub")]
    repository: String,

    #[clap(long, short, group = "sub")]
    all: bool,
}

pub async fn execute(args: FetchArgs) {
    println!("fetching from {}", args.repository);
    if args.all {
        let remotes = Config::remote_configs()
            .await
            .iter()
            .map(|x| x.name.clone())
            .collect::<HashSet<String>>();
        
        let tasks = remotes.into_iter().map(|remote| async move {
            fetch_repository(&remote).await;
        });
        futures::future::join_all(tasks).await;
    } else {
        fetch_repository(&args.repository).await;
    }
}

async fn fetch_repository(remote: &str) {
    println!("fetching from {}", remote);
    unimplemented!("")
}
