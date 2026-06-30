//! Mono is an engine for managing a monorepo. It functions similarly to Google's Piper and helps to streamline Git
//! and trunk-based development for large-scale projects.
//!
//! And this is the main entry point for the application.

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL_ALLOCATOR: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    if let Err(e) = mono::cli::parse(None) {
        panic!("{}", e);
    }
}
