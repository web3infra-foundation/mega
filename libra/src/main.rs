//! This is the main entry point for the Libra.

use libra::cli;
use mercury::errors::GitError;

fn main() {
    #[cfg(debug_assertions)]
    {
        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .finish(),
        )
        .unwrap();
    }

    let res = cli::parse(None);
    match res {
        Ok(_) => {}
        Err(e) => {
            if !matches!(e, GitError::RepoNotFound) {
                eprintln!("Error: {:?}", e);
            }
        }
    }
}
