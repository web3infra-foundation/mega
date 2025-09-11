//! the core controller module
//! It receives messages from Kafka MQ one by one,
//! parse them, and store it into tugraph, and notify
//! other processes.

//use analysis::analyse_once;
#[allow(unused_imports)]
use repo_import::ImportDriver;

use crate::cli::CratesProCli;
use futures_util::future::FutureExt;
//use std::process::Command;
//use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
#[allow(unused_imports)]
use std::{env, fs, sync::Arc, time::Duration};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::Mutex;

pub struct CoreController {
    pub cli: CratesProCli,

    pub import: bool,
}
struct SharedState {
    is_packaging: bool,
}

impl CoreController {
    pub async fn new(cli: CratesProCli) -> Self {
        let import = env::var("CRATES_PRO_IMPORT").unwrap().eq("1");
        Self {
            cli,
            import,
        }
    }

    pub async fn run(&self) {
        let import = self.import;

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
                            "Import task received SIGTERM, will exit after current message"
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
                    if count == 20000 {
                        import_driver.context.write_tugraph_import_files().await;
                        let _ =import_driver.context.import_from_env_vars().await;
                        count = 0;
                    }
                    drop(state);

                    if result.is_err() {
                        tokio::time::sleep(Duration::from_secs(0)).await;
                    }
                }
            }
        });

        // 只等待实际运行的任务
        if import {
            import_task.await.unwrap();
            tracing::info!("Import task completed");
        }
    }
}
