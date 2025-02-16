use std::fmt;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;
use async_channel::{Receiver, Sender};
use common::config::Config;
use jupiter::context::Context as MegaContext;
use crate::application::Action;
use crate::config::MEGA_CONFIG_PATH;
use crate::core::{init_mega_log, load_mega_resource};
use crate::core::servers::{HttpOptions, SshOptions};
use crate::error::{MonoBeanError, MonoBeanResult};

pub struct MegaCore {
    config: Config,
    running_context: Option<MegaContext>,
    mount_point: Option<PathBuf>,
    ssh_options: Option<SshOptions>,
    http_options: Option<HttpOptions>,

    mounted: bool,

    sender: Sender<Action>,
    receiver: Receiver<MegaCommands>,
}

#[derive(Debug, Clone)]
pub enum MegaCommands {
    // Mega Backend Related Actions
    // These actions will be transfer to the mega core event loop.
    MegaStart(Option<SocketAddr>, Option<SocketAddr>),
    MegaShutdown,
    MegaRestart(Option<SocketAddr>, Option<SocketAddr>),
    FuseMount(PathBuf),
    FuseUnmount,
    SaveFileChange(PathBuf),
}

impl Debug for MegaCore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("MegaCore")
            .field("config", &self.config)
            .field("mount_point", &self.mount_point)
            .field("mounted", &self.mounted)
            .finish()
    }
}

impl MegaCore {
    pub fn new(sender: Sender<Action>, receiver: Receiver<MegaCommands>) -> Self {
        let bytes = load_mega_resource(MEGA_CONFIG_PATH);
        let content = String::from_utf8(bytes).expect("Mega core setting must be in utf-8");
        let config =
            Config::load_str(content.as_str()).expect("Failed to parse mega core settings");

        init_mega_log(&config.log);

        Self {
            config,
            running_context: Default::default(),
            mount_point: None,
            ssh_options: None,
            http_options: None,

            mounted: false,
            sender,
            receiver,
        }
    }

    /// Entry point of Mega Core.
    /// ## Warning:
    /// This function must be called in a tokio runtime.
    pub async fn process_commands(&mut self) {
        while let Ok(cmd) = self.receiver.recv().await {
            match cmd {
                MegaCommands::MegaStart(http_addr, ssh_addr) => {
                    tracing::info!("Starting Mega Core");
                    self.launch(http_addr, ssh_addr).await.unwrap();
                }
                MegaCommands::MegaShutdown => {
                    tracing::info!("Shutting down Mega Core");
                    self.shutdown();
                }
                MegaCommands::MegaRestart(http_addr, ssh_addr) => {
                    tracing::info!("Restarting Mega Core");
                    self.shutdown();
                    self.launch(http_addr, ssh_addr).await.unwrap();
                }
                MegaCommands::FuseMount(path) => {
                    tracing::info!("Mounting fuse at {:?}", path);
                    self.mount_point = Some(path);
                    self.mounted = true;
                }
                MegaCommands::FuseUnmount => {
                    tracing::info!("Unmounting fuse");
                    self.mount_point = None;
                    self.mounted = false;
                }
                MegaCommands::SaveFileChange(path) => {
                    tracing::info!("Saving file change at {:?}", path);
                }
            }
        }
    }
    async fn launch(
        &mut self,
        http_addr: Option<SocketAddr>,
        ssh_addr: Option<SocketAddr>,
    ) -> MonoBeanResult<()> {
        if self.is_core_running() {
            let inner = MegaContext::new(self.config.clone()).await;
            inner
                .services
                .mono_storage
                .init_monorepo(&self.config.monorepo)
                .await;

            self.running_context = Some(inner);
            self.http_options = http_addr.map(HttpOptions::new).or(None);
            self.ssh_options = ssh_addr.map(SshOptions::new).or(None);
        } else {
            let err = "Mega core is already running";
            tracing::error!(err);
            return Err(MonoBeanError::MegaCoreError(err.to_string()));
        }

        // Affordable tradeoff for convenience
        let http_clone = self.http_options.clone().unwrap();
        let ssh_clone = self.ssh_options.clone().unwrap();
        let (http_res, ssh_res) = tokio::join!(
            http_clone.run_server(self.running_context.clone().unwrap()),
            ssh_clone.run_server(self.running_context.clone().unwrap())
        );

        let _ = http_res.map_err(|e| {
            MonoBeanError::MegaCoreError(format!("Failed to serve http: {}", e.to_string()))
        })?;
        let _ = ssh_res.map_err(|e| {
            MonoBeanError::MegaCoreError(format!("Failed to serve ssh: {}", e.to_string()))
        })?;
        Ok(())
    }

    fn shutdown(&mut self) {
        if let Some(http_options) = self.http_options.as_ref() {
            http_options.shutdown_server();
        }
        if let Some(ssh_options) = self.ssh_options.as_ref() {
            ssh_options.shutdown_server();
        }
        self.running_context = None;
    }

    pub fn is_core_running(&self) -> bool {
        self.running_context.is_some()
    }

    pub fn update_network_options(
        &mut self,
        http_options: Option<HttpOptions>,
        ssh_options: Option<SshOptions>,
    ) {
        if let Some(http_options) = http_options {
            self.http_options = Some(http_options);
        }
        if let Some(ssh_options) = ssh_options {
            self.ssh_options = Some(ssh_options);
        }
    }
}