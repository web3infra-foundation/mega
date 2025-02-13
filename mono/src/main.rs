//! Mono is an engine for managing a monorepo. It functions similarly to Google's Piper and helps to streamline Git
//! and trunk-based development for large-scale projects.
//!
//! And this is the main entry point for the application.

use shadow_rs::shadow;
shadow!(build);

mod cli;
mod commands;

pub mod api;
pub mod git_protocol;
pub mod server;

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    // Parse the command line arguments
    let result = cli::parse(None);

    // If there was an error, print it
    if let Err(e) = result {
        e.print();
        eprintln!("Version:{}", build::VERSION);
        eprintln!("Version:{}", build::CLAP_LONG_VERSION);
        eprintln!("Version:{}", build::PKG_VERSION);
        eprintln!("OS:{}", build::BUILD_OS);
        eprintln!("Rust Version:{}", build::RUST_VERSION);
        eprintln!("Rust Channel:{}", build::RUST_CHANNEL);
        eprintln!("Cargo Version:{}", build::CARGO_VERSION);
        eprintln!("Build Time:{}", build::BUILD_TIME);
    }
}
