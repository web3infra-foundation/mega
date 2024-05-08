use clap::{Parser, Subcommand};

mod command;
mod db;
mod internal;
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
    #[command(about = "Show the working tree status")]
    Status,
    #[command(about = "List, create, or delete branches")]
    Branch(command::branch::BranchArgs),
    #[command(about = "Remove files from the working tree and from the index")]
    Rm(command::remove::RemoveArgs),
    #[command(about = "Show commit logs")]
    Log(command::log::LogArgs),
    #[command(about = "Restore working tree files")]
    Restore(command::restore::RestoreArgs),
    #[command(about = "Switch branches")]
    Switch(command::switch::SwitchArgs),
    #[command(about = "Clone a repository into a new directory")]
    Clone(command::clone::CloneArgs),
    #[command(about = "Build pack index file for an existing packed archive", hide = true)]
    IndexPack(command::index_pack::IndexPackArgs),
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    // check repo existence, except for `init` and `clone`
    // TODO: try check repo before parsing
    if let Commands::Init = args.command {
    } else if let Commands::Clone(_) = args.command {
    } else if !utils::util::check_repo_exist() {
        return;
    }

    // parse the command and execute the corresponding function with it's args
    match args.command {
        Commands::Init => command::init::execute().await,
        Commands::Commit(args) => command::commit::execute(args).await,
        Commands::Add(args) => command::add::execute(args).await,
        Commands::Status => command::status::execute().await,
        Commands::Branch(args) => command::branch::execute(args).await,
        Commands::Rm(args) => command::remove::execute(args).unwrap(),
        Commands::Log(args) => command::log::execute(args).await,
        Commands::Restore(args) => command::restore::execute(args).await,
        Commands::Switch(args) => command::switch::execute(args).await,
        Commands::Clone(args) => command::clone::execute(args).await,
        Commands::IndexPack(args) => command::index_pack::execute(args),
    }
}

/// this test is to verify that the CLI can be built without panicking
/// according [clap dock](https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_4/index.html)
#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
