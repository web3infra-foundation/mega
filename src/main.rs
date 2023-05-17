//!
//!
//!
//!
//!
mod errors;
mod cli;
mod commands;
mod utils;

fn main() {
    // Parse the command line arguments
    let result = cli::parse();

    // If there was an error, print it
    if let Err(e) = result {
        e.print()
    }
}
