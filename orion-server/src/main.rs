use std::io::IsTerminal;
/// Orion Build Server
/// A distributed build system that manages build tasks and worker nodes
use std::path::PathBuf;

use common::config::{
    Config,
    loader::{ConfigInput, ConfigLoader},
};

fn tracing_level_from_config(level: &str) -> tracing::Level {
    match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    }
}

fn init_tracing_from_config() {
    let input = ConfigInput {
        cli_path: None,
        env_path: std::env::var_os("MEGA_CONFIG").map(PathBuf::from),
    };
    let (level, with_ansi) = match ConfigLoader::new(input).load() {
        Ok(loaded) => match loaded.path.to_str() {
            Some(p) => match Config::new(p) {
                Ok(cfg) => (
                    tracing_level_from_config(&cfg.log.level),
                    cfg.log.with_ansi && std::io::stdout().is_terminal(),
                ),
                Err(e) => {
                    eprintln!("Failed to parse config for log level: {e}; using info");
                    (tracing::Level::INFO, std::io::stdout().is_terminal())
                }
            },
            None => {
                eprintln!("Config path is not valid UTF-8; using info");
                (tracing::Level::INFO, std::io::stdout().is_terminal())
            }
        },
        Err(e) => {
            eprintln!("Failed to locate config for log level: {e}; using info");
            (tracing::Level::INFO, std::io::stdout().is_terminal())
        }
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(with_ansi)
        .init();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_tracing_from_config();

    orion_server::server::start_server().await;
}
