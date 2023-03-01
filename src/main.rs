//!
//!
//!
//!
//!
mod errors;
mod cli;
mod commands;

fn main() {
    // Parse the command line arguments
    let result = cli::parse();

    // If there was an error, print it
    if let Err(e) = result {
        e.print()
    }
}
