//!
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
