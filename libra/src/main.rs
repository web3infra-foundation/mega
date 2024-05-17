use clap::{Parser, Subcommand};
mod command;
mod internal;
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
    // start a working area
    #[command(about = "Initialize a new repository")]
    Init,
    #[command(about = "Clone a repository into a new directory")]
    Clone(command::clone::CloneArgs),

    // work on the current change
    #[command(about = "Add file contents to the index")]
    Add(command::add::AddArgs),
    #[command(about = "Remove files from the working tree and from the index")]
    Rm(command::remove::RemoveArgs),
    #[command(about = "Restore working tree files")]
    Restore(command::restore::RestoreArgs),

    // examine the history and state
    #[command(about = "Show the working tree status")]
    Status,
    #[command(about = "Show commit logs")]
    Log(command::log::LogArgs),

    // grow, mark and tweak your common history
    #[command(about = "List, create, or delete branches")]
    Branch(command::branch::BranchArgs),
    #[command(about = "Record changes to the repository")]
    Commit(command::commit::CommitArgs),
    #[command(about = "Switch branches")]
    Switch(command::switch::SwitchArgs),
    #[command(about = "Merge changes")]
    Merge(command::merge::MergeArgs),

    // collaborate
    // todo: implement in the future
    #[command(about = "Update remote refs along with associated objects")]
    Push(command::push::PushArgs),
    #[command(about = "Download objects and refs from another repository")]
    Fetch(command::fetch::FetchArgs),
    #[command(about = "Fetch from and integrate with another repository or a local branch")]
    Pull(command::pull::PullArgs),

    #[command(subcommand, about = "Manage set of tracked repositories")]
    Remote(command::remote::RemoteCmds),

    // other hidden commands
    #[command(
        about = "Build pack index file for an existing packed archive",
        hide = true
    )]
    IndexPack(command::index_pack::IndexPackArgs),
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    // TODO: try check repo before parsing
    if let Commands::Init = args.command {
    } else if let Commands::Clone(_) = args.command {
    } else if !utils::util::check_repo_exist() {
        return;
    }

    #[cfg(debug_assertions)]
    {
        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )
        .unwrap();
    }
    // parse the command and execute the corresponding function with it's args
    match args.command {
        Commands::Init => command::init::execute().await,
        Commands::Clone(args) => command::clone::execute(args).await,
        Commands::Add(args) => command::add::execute(args).await,
        Commands::Rm(args) => command::remove::execute(args).unwrap(),
        Commands::Restore(args) => command::restore::execute(args).await,
        Commands::Status => command::status::execute().await,
        Commands::Log(args) => command::log::execute(args).await,
        Commands::Branch(args) => command::branch::execute(args).await,
        Commands::Commit(args) => command::commit::execute(args).await,
        Commands::Switch(args) => command::switch::execute(args).await,
        Commands::Merge(args) => command::merge::execute(args).await,
        Commands::Push(args) => command::push::execute(args).await,
        Commands::IndexPack(args) => command::index_pack::execute(args),
        Commands::Fetch(args) => command::fetch::execute(args).await,
        Commands::Remote(cmd) => command::remote::execute(cmd).await,
        Commands::Pull(args) => command::pull::execute(args).await,
    }
}

/// this test is to verify that the CLI can be built without panicking
/// according [clap dock](https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_4/index.html)
#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
