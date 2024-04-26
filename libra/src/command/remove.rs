use clap::Parser;

#[derive(Parser, Debug)]
pub struct RemoveArgs {
    /// file or dir to remove
    pathspec: Vec<String>,
    /// whether to remove from index
    #[clap(long, action)]
    cached: bool,
    /// indicate recursive remove dir
    #[clap(short, long)]
    recursive: bool,
}

pub fn execute(args: RemoveArgs) {
    // TODO
}