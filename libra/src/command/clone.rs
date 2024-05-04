use clap::Parser;

#[derive(Parser, Debug)]
pub struct CloneArgs {
    /// The remote repository location to clone from, usually a URL with HTTPS or SSH
    #[clap(long, short)]
    pub remote_repo: String,

    /// The local path to clone the repository to
    #[clap(long, short)]
    pub local_path: String,
}

pub async fn execute(args: CloneArgs) {
    println!(
        "Cloning repository from {} to {}",
        args.remote_repo, args.local_path
    );
    println!("Not implemented yet")
}
