use clap::Parser;

#[derive(Parser, Debug)]
pub struct MergeArgs {
    /// The branch to merge into the current branch
    pub branch: String,
}

pub async fn execute(args: MergeArgs) {
    let branch = args.branch;
    println!("Merging branch {}", branch);
    println!("Not yet implemented")
}
