use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Sets a custom workspace
    #[arg(short = 'p', long, value_name = "FILE")]
    pub workspace: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Push Local Repo to mega
    Upload,
    /// Async Crate to Repo
    Crate,
    /// Incremental Update. the <FILE> arg is useless.
    Incremental,
    /// Sync Crate to Repo
    Sync,
}
