//! Mega is an engine for managing a monorepo. It functions similarly to Google's Piper and helps to streamline Git
//! and trunk-based development for large-scale projects. And this is the main entry point for the application.

use std::env;

use tracing_subscriber::fmt::writer::MakeWriterExt;

mod cli;
mod commands;

fn main() {
    dotenvy::dotenv().ok();

    let file_appender =
        tracing_appender::rolling::hourly(env::var("MEGA_LOG_PATH").unwrap(), "mega-logs");
    let stdout = std::io::stdout;

    let log_level = match env::var("RUST_LOG").unwrap().as_str() {
        "TRACE" => tracing::Level::TRACE,
        "DEBUG" => tracing::Level::DEBUG,
        "INFO" => tracing::Level::INFO,
        "WARN" => tracing::Level::WARN,
        "ERROR" => tracing::Level::ERROR,
        _ => unreachable!("Invalid log level"),
    };

    tracing_subscriber::fmt()
        .with_writer(stdout.and(file_appender))
        .with_max_level(log_level)
        .init();

    // Parse the command line arguments
    let result = cli::parse();

    // If there was an error, print it
    if let Err(e) = result {
        e.print()
    }
}
