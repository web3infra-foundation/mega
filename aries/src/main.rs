use clap::Parser;
use common::config::{Config, LogConfig};
use service::relay_server::{run_relay_server, RelayOptions};
use std::{env, path::PathBuf};
use tracing_subscriber::fmt::writer::MakeWriterExt;

pub mod service;

#[tokio::main]
async fn main() {
    ctrlc::set_handler(move || {
        tracing::info!("Received Ctrl-C signal, exiting...");
        std::process::exit(0);
    })
    .unwrap();

    let option = RelayOptions::parse();
    let config_path = PathBuf::from(
        option
            .config
            .to_owned()
            .unwrap_or("config.toml".to_string()),
    );
    let config = if config_path.exists() {
        Config::new(config_path.to_str().unwrap()).unwrap()
    } else {
        eprintln!("can't find config.toml under {:?}, you can manually set config.toml path with --config parameter", env::current_dir().unwrap());
        Config::default()
    };

    init_log(&config.log);

    tracing::info!("{:?}", option);

    //Start  relay server
    run_relay_server(config, option).await;
}

fn init_log(config: &LogConfig) {
    let log_level = match config.level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let file_appender =
        tracing_appender::rolling::hourly(config.log_path.clone(), "mega-relay-logs");

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
