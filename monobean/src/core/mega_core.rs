use crate::application::Action;
use crate::config::MEGA_CONFIG_PATH;
use crate::core::load_mega_resource;
use crate::core::servers::{HttpOptions, SshOptions};
use crate::error::{MonoBeanError, MonoBeanResult};
use async_channel::{Receiver, Sender};
use common::config::Config;
use jupiter::context::Context as MegaContext;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::sync::{oneshot, OnceCell};
use vault::pgp::{SignedPublicKey, SignedSecretKey};

pub struct MegaCore {
    config: Config,
    running_context: Option<MegaContext>,
    mount_point: Option<PathBuf>,
    ssh_options: Option<SshOptions>,
    http_options: Option<HttpOptions>,
    pgp: OnceCell<(SignedPublicKey, SignedSecretKey)>,

    mounted: bool,

    #[allow(dead_code)]
    sender: Sender<Action>,
    pub receiver: Receiver<MegaCommands>,
}

/// Mega Backend Related Actions
#[derive(Debug)]
pub enum MegaCommands {
    MegaStart(Option<SocketAddr>, Option<SocketAddr>),
    MegaShutdown,
    MegaRestart(Option<SocketAddr>, Option<SocketAddr>),
    // (core_running, pgp_initialized)
    CoreStatus(oneshot::Sender<(bool, bool)>),
    FuseMount(PathBuf),
    FuseUnmount,
    SaveFileChange(PathBuf),
    LoadOrInitPgp(
        oneshot::Sender<MonoBeanResult<()>>,
        String,         // User Name
        String,         // User Email
        Option<String>, // Passwd
    ),
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

        Self {
            config,
            running_context: Default::default(),
            mount_point: None,
            ssh_options: None,
            http_options: None,
            pgp: Default::default(),

            mounted: false,
            sender,
            receiver,
        }
    }

    /// Processes a given `MegaCommands` command.
    ///
    /// # Arguments
    ///
    /// * `cmd` - A `MegaCommands` enum variant representing the command to be processed.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut mega_core = MegaCore::new(sender, receiver);
    /// mega_core.process_command(MegaCommands::MegaStart(None, None)).await;
    /// ```
    ///
    /// # Warning
    ///
    /// This function must be called in a tokio runtime.
    pub(crate) async fn process_command(&mut self, cmd: MegaCommands) {
        // FIXME: for command with callback channel, detect if `send` success.
        match cmd {
            MegaCommands::MegaStart(http_addr, ssh_addr) => {
                tracing::info!("Starting Mega Core");
                self.init().await;
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
            MegaCommands::CoreStatus(sender) => {
                let core_running = self.is_core_running();
                let pgp_initialized = self.pgp.initialized();
                sender.send((core_running, pgp_initialized)).unwrap();
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
            MegaCommands::LoadOrInitPgp(back_chan, name, email, passwd) => {
                if self.pgp.initialized() {
                    back_chan.send(Err(MonoBeanError::ReinitError)).unwrap();
                    return;
                }

                if let Some(pk) = vault::pgp::load_pub_key().await {
                    let sk = vault::pgp::load_sec_key().await.unwrap();
                    back_chan.send(Ok(())).unwrap();
                    self.pgp.set((pk, sk)).unwrap();
                } else {
                    let uid = format!("{} <{}>", name, email);
                    let params = vault::pgp::params(
                        vault::pgp::KeyType::Rsa(2048),
                        passwd.clone(),
                        uid.as_ref(),
                    );
                    let (pk, sk) = vault::pgp::gen_pgp_keypair(params, passwd);
                    vault::pgp::save_keys(pk.clone(), sk.clone()).await;
                    back_chan.send(Ok(())).unwrap();
                    self.pgp.set((pk, sk)).unwrap();
                }
            }
        }
    }

    /// Initialize MegaCore at startup phrase.
    async fn init(&mut self) {
        // Try to load pgp keys from vault.
        if let Some(pk) = vault::pgp::load_pub_key().await {
            let sk = vault::pgp::load_sec_key().await.unwrap();
            self.pgp.set((pk, sk)).unwrap();
            tracing::debug!("Loaded pgp keys from vault");
        }
    }

    /// Launch Mega Http(s) and SSH servers.
    async fn launch(
        &mut self,
        http_addr: Option<SocketAddr>,
        ssh_addr: Option<SocketAddr>,
    ) -> MonoBeanResult<()> {
        if !self.is_core_running() {
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

        http_res.map_err(|e| {
            MonoBeanError::MegaCoreError(format!("Failed to serve http: {}", e))
        })?;
        ssh_res.map_err(|e| {
            MonoBeanError::MegaCoreError(format!("Failed to serve ssh: {}", e))
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
}
