use std::sync::OnceLock;
use adw::gio;
use adw::gio::ResourceLookupFlags;
use adw::prelude::*;
use tokio::runtime::Runtime;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use common::config::LogConfig;

pub mod delegate;
pub mod servers;
pub mod mega_core;

// For running mega core, we should set up tokio runtime.
pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .build()
            .expect("Setting up tokio runtime must succeed.")
    })
}

pub fn load_mega_resource(path: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = gio::resources_open_stream(path, ResourceLookupFlags::all())
        .expect("Failed to load mega core settings")
        .read_all(&mut bytes, gio::Cancellable::NONE)
        .expect("Failed to read mega core settings");
    bytes
}

// TODO: move to `application.rs` to globally initialize the log
fn init_mega_log(config: &LogConfig) {
    let log_level = match config.level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let file_appender = tracing_appender::rolling::hourly(config.log_path.clone(), "monobean-logs");

    if config.print_std {
        let stdout = std::io::stdout;
        tracing_subscriber::fmt()
            .with_writer(stdout.and(file_appender))
            .with_max_level(log_level)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_max_level(log_level)
            .init();
    }
}
