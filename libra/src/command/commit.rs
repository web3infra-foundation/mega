use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Record changes to the repository")]
pub struct CommitArgs {
    #[arg(short, long)]
    pub message: String,

    #[arg(long)]
    pub allow_empty: bool,
}

pub async fn execute(args: CommitArgs) {
    println!("Committing with message: '{}'", args.message);
    println!("Not yet implemented");
}
