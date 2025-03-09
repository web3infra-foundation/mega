//! Mega is an engine for managing a monorepo. It functions similarly to Google's Piper and helps to streamline Git
//! and trunk-based development for large-scale projects. And this is the main entry point for the application.

mod cli;
mod commands;

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL_ALLOCATOR: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    // Parse the command line arguments
    let result = cli::parse(None);

    // If there was an error, print it
    if let Err(e) = result {
        e.print();
    }
}
