use clap::Parser;
use common::config::{Config, LogConfig};
use gemini::ztm::hub::LocalZTMHub;
use service::{
    ca_server::run_ca_server,
    relay_server::{run_relay_server, RelayOptions},
};
use std::{env, thread, time};
use tracing_subscriber::fmt::writer::MakeWriterExt;

pub mod service;

#[tokio::main]
async fn main() {
    // Get the current directory
    let current_dir = env::current_dir().unwrap();
    // Get the path to the config file in the current directory
    let config_path = current_dir.join("config.toml");

    let config = if config_path.exists() {
        Config::new(config_path.to_str().unwrap()).unwrap()
    } else {
        eprintln!("can't find config.toml under {:?}, you can manually set config.toml path with --config parameter", env::current_dir().unwrap());
        Config::default()
    };

    init_log(&config.log);

    ctrlc::set_handler(move || {
        tracing::info!("Received Ctrl-C signal, exiting...");
        std::process::exit(0);
    })
    .unwrap();

    let option = RelayOptions::parse();
    tracing::info!("{:?}", option);

    //Start a sub thread to ca server
    let config_clone = config.clone();
    let ca_port = option.ca_port;
    tokio::spawn(async move { run_ca_server(config_clone, ca_port).await });
    thread::sleep(time::Duration::from_secs(5));

    //Start a sub thread to run ztm-hub
    let ca = format!("127.0.0.1:{ca_port}");
    let ztm_hub: LocalZTMHub = LocalZTMHub {
        hub_port: option.ztm_hub_port,
        ca,
        name: vec!["relay".to_string()],
    };
    ztm_hub.clone().start_ztm_hub();
    thread::sleep(time::Duration::from_secs(5));

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
