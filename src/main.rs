//! Mega is an engine for managing a monorepo. It functions similarly to Google's Piper and helps to streamline Git 
//! and trunk-based development for large-scale projects.
//!
//!
//!
//!

use std::env;
mod cli;
mod commands;
mod utils;

fn main() {
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    // Parse the command line arguments
    let result = cli::parse();

    // If there was an error, print it
    if let Err(e) = result {
        e.print()
    }
}
