use clap::{Parser, Subcommand};

mod command;
mod db;
mod model;
mod utils;

#[derive(Parser, Debug)]
#[command(about = "Simulates git commands", version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Libra sub commands, similar to git
/// subcommands's excute and args are defined in `command` module
#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Initialize a new repository")]
    Init,
    #[command(about = "Record changes to the repository")]
    Commit(command::commit::CommitArgs),
    #[command(about = "Add file contents to the index")]
    Add(command::add::AddArgs),
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    // parse the command and execute the corresponding function with it's args
    match args.command {
        Commands::Init => command::init::execute().await,
        Commands::Commit(args) => command::commit::execute(args).await,
        Commands::Add(args) => command::add::execute(args).await,
    }
}

/// this test is to verify that the CLI can be built without panicking
/// according [clap dock](https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_4/index.html)
#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
