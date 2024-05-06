use clap::Parser;

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

    let url = url::Url::parse(&remote_repo).unwrap();
}
