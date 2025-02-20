use clap::Parser;
use common::config::{Config, LogConfig};
use gemini::ztm::{
    agent::{run_ztm_client, LocalZTMAgent, ZTMAgent},
    hub::LocalZTMHub,
};
use service::{
    ca_server::run_ca_server,
    relay_server::{run_relay_server, RelayOptions},
};
use std::{
    env,
    path::PathBuf,
    thread::{self},
    time::{self},
};
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

    if option.only_agent {
        let (peer_id, _) = vault::init().await;
        let ztm_agent: LocalZTMAgent = LocalZTMAgent {
            agent_port: option.ztm_agent_port,
        };
        ztm_agent.clone().start_ztm_agent();
        thread::sleep(time::Duration::from_secs(3));
        run_ztm_client(
            "http://gitmono.org/relay".to_string(),
            config.clone(),
            peer_id,
            ztm_agent,
            8001,
        )
        .await
    }

    //Start a sub thread to ca server
    let config_clone = config.clone();
    let ca_port = option.ca_port;
    tokio::spawn(async move { run_ca_server(config_clone, ca_port).await });
    thread::sleep(time::Duration::from_secs(3));

    //Start a sub thread to run ztm-hub
    let ca = format!("127.0.0.1:{ca_port}");
    let ztm_hub: LocalZTMHub = LocalZTMHub {
        hub_port: option.ztm_hub_port,
        ca,
        name: vec!["relay".to_string()],
    };
    ztm_hub.clone().start_ztm_hub();
    thread::sleep(time::Duration::from_secs(3));

    //Start a sub thread to run ztm-agent
    let ztm_agent = LocalZTMAgent {
        agent_port: option.ztm_agent_port,
    };
    thread::sleep(time::Duration::from_secs(3));

    match ztm_agent.get_ztm_endpoints().await {
        Ok(ztm_ep_list) => {
            tracing::info!("ztm agent connect success");
            tracing::info!("{} online endpoints", ztm_ep_list.len());
        }
        Err(_) => {
            tracing::error!("ztm agent connect failed");
        }
    }

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
