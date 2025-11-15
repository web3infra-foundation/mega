//! the core controller module
//! It receives messages from Kafka MQ one by one,
//! parse them, and store it into tugraph, and notify
//! other processes.


use data_transporter::Transporter;
//use neo4rs::{ConfigBuilder, Graph};
//use analysis::analyse_once;
#[allow(unused_imports)]
use repo_import::ImportDriver;

use crate::cli::CratesProCli;
use futures_util::future::FutureExt;
//use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
#[allow(unused_imports)]
use std::{env, fs, sync::Arc, time::Duration};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::Mutex;

pub struct CoreController {
    pub cli: CratesProCli,

    pub import: bool,
    pub package: bool,
}
struct SharedState {
    is_packaging: bool,
}

impl CoreController {
    pub async fn new(cli: CratesProCli) -> Self {
        let import = env::var("CRATES_PRO_IMPORT").unwrap().eq("1");
        let package = env::var("CRATES_PRO_PACKAGE").unwrap().eq("1");
        Self {
            cli,
            import,
            package,
        }
    }

    pub async fn run(&self) {
        let import = self.import;
        let package = self.package;

        let shared_state: Arc<tokio::sync::Mutex<SharedState>> =
            Arc::new(Mutex::new(SharedState {
                is_packaging: false,
            }));

        let dont_clone = self.cli.dont_clone;
        let state_clone1: Arc<tokio::sync::Mutex<SharedState>> = Arc::clone(&shared_state);
        let import_task = tokio::spawn(async move {
            if import {
                let should_reset_kafka_offset =
                    env::var("SHOULD_RESET_KAFKA_OFFSET").unwrap().eq("1");
                if should_reset_kafka_offset {
                    repo_import::reset_kafka_offset()
                        .await
                        .unwrap_or_else(|x| panic!("{}", x));
                }

                let mut import_driver = ImportDriver::new(dont_clone).await;
                //import_driver.context.version_updater.version_parser.version_map.entry("serde_json/1.0".to_string()).or_default().push("".to_string());
                let mut count = 0;
                let is_importing = Arc::new(AtomicBool::new(false));
                let is_importing_clone = Arc::clone(&is_importing);
                let mut term_signal = signal(SignalKind::terminate()).unwrap();
                let mut received_term = false;

                loop {
                    let mut state = state_clone1.lock().await;
                    while state.is_packaging {
                        drop(state);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        state = state_clone1.lock().await;
                    }

                    is_importing_clone.store(true, Ordering::SeqCst);
                    let result = import_driver.import_from_mq_for_a_message().await;
                    is_importing_clone.store(false, Ordering::SeqCst);

                    if !received_term && term_signal.recv().now_or_never().is_some() {
                        tracing::info!(
                            "Import task received SIGTERM:{:?}, will exit after current message",term_signal
                        );
                        received_term = true;
                    }

                    if received_term {
                        tracing::info!("Import task saving final checkpoint...");
                        if let Err(e) = import_driver.save_checkpoint().await {
                            tracing::error!("Failed to save final checkpoint: {}", e);
                        } else {
                            tracing::info!("Final checkpoint saved successfully");
                        }
                        std::process::exit(0);
                    }

                    count += 1;
                    if count == 100000 {
                        
                        tracing::info!("Import task saving checkpoint...");
                        if let Err(e) = import_driver.save_checkpoint().await {
                            tracing::error!("Failed to save checkpoint: {}", e);
                        } else {
                            tracing::info!("checkpoint saved successfully");
                        }
                        tracing::info!("finish save checkpoint");
                        

                        count = 0;
                    }
                    drop(state);

                    if result.is_err() {
                        tokio::time::sleep(Duration::from_secs(0)).await;
                    }
                }
            }
        });
        let state_clone3: Arc<tokio::sync::Mutex<SharedState>> = Arc::clone(&shared_state);
        let package_task = tokio::spawn(async move {
            if package {
                loop {
                    {
                        let mut state = state_clone3.lock().await;
                        state.is_packaging = true;
                    }

                    // process here

                    {
                        let tugraph_bolt_url = env::var("TUGRAPH_BOLT_URL").expect("TUGRAPH_BOLT_URL environment variable must be set");
                        let tugraph_user_name = env::var("TUGRAPH_USER_NAME").expect("TUGRAPH_USER_NAME environment variable must be set");
                        let tugraph_user_password = env::var("TUGRAPH_USER_PASSWORD").expect("TUGRAPH_USER_PASSWORD environment variable must be set");
                        let tugraph_cratespro_db = env::var("TUGRAPH_CRATESPRO_DB").expect("TUGRAPH_CRATESPRO_DB environment variable must be set");
                        let mut transporter = Transporter::new(
                            &tugraph_bolt_url,
                            &tugraph_user_name,
                            &tugraph_user_password,
                            &tugraph_cratespro_db,
                        )
                        .await;

                        if let Err(e) = transporter.transport_data().await {
                            tracing::error!("Failed to transport data: {:?}", e);
                        }
                    }

                    {
                        let mut state = state_clone3.lock().await;
                        state.is_packaging = false;
                    }

                    // after one day
                    tokio::time::sleep(Duration::from_secs(86400)).await;
                }
            }
        });

        // 只等待实际运行的任务
        if import {
            import_task.await.unwrap();
            tracing::info!("Import task completed");
        }
        if package {
            package_task.await.unwrap();
            tracing::info!("Package task completed");
        }
    }
}
