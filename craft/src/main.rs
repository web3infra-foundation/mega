//!
//!
//!
//!
//!

use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};

use git_craft::vault::{self, command::VaultArgs};

mod lfs;
use crate::lfs::command::LfsArgs;

#[derive(Parser, Debug)]
#[command(
    version = "0.1.0",
    about,
    long_about = "Usage: generate-key, generate-key-full [primary_id] [key_name], encrypt [public_key_path], decrypt [secret_key_path], list-keys , delete-key [key_name]"
)]
struct CraftOptions {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Vault(VaultArgs),
    Lfs(LfsArgs),
    P2p,
}

// Program main function
// Arguments: accept command line arguments.
fn main() -> Result<(), anyhow::Error> {
    // Collect command line arguments into Args
    let args = CraftOptions::parse();
    match args.command {
        Commands::Vault(args) => vault::command::handle(args),
        Commands::Lfs(args) => lfs::command::handle(args),
        Commands::P2p => todo!(),
    }
    Ok(())
}
