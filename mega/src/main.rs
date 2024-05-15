//! Mega is an engine for managing a monorepo. It functions similarly to Google's Piper and helps to streamline Git
//! and trunk-based development for large-scale projects. And this is the main entry point for the application.

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
