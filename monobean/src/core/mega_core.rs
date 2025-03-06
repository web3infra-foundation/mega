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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, OnceCell, RwLock};
use vault::pgp::{SignedPublicKey, SignedSecretKey};

pub struct MegaCore {
    config: Arc<Config>,
    running_context: Arc<RwLock<Option<MegaContext>>>,
    mount_point: Arc<RwLock<Option<PathBuf>>>,
    ssh_options: Arc<RwLock<Option<SshOptions>>>,
    http_options: Arc<RwLock<Option<HttpOptions>>>,
    pgp: OnceCell<(SignedPublicKey, SignedSecretKey)>,

    initialized: AtomicBool,
    mounted: AtomicBool,

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
    CoreStatus(
        oneshot::Sender<(
            /* core_running: */ bool,
            /* pgp_initialized: */ bool,
        )>,
    ),
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
            config: Arc::from(config),
            running_context: Default::default(),
            mount_point: Default::default(),
            ssh_options: Default::default(),
            http_options: Default::default(),
            pgp: Default::default(),

            initialized: Default::default(),
            mounted: Default::default(),
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
    ///
    /// # Deadlock (For Developers)
    ///
    /// Should not block sending an `Action` in main thread, or the code should be put in a `tokio::spawn` block.
    pub(crate) async fn process_command(&self, cmd: MegaCommands) {
        // FIXME: for command with callback channel, detect if `send` success.
        tracing::debug!("Processing command: {:?}", cmd);
        match cmd {
            MegaCommands::MegaStart(http_addr, ssh_addr) => {
                tracing::info!("Starting Mega Core");
                self.launch(http_addr, ssh_addr).await.unwrap();
            }
            MegaCommands::MegaShutdown => {
                tracing::info!("Shutting down Mega Core");
                self.shutdown().await;
            }
            MegaCommands::MegaRestart(http_addr, ssh_addr) => {
                tracing::info!("Restarting Mega Core");
                self.shutdown().await;
                self.launch(http_addr, ssh_addr).await.unwrap();
            }
            MegaCommands::CoreStatus(sender) => {
                let core_running = self.is_core_running().await;
                let pgp_initialized = self.pgp.initialized();
                sender.send((core_running, pgp_initialized)).unwrap();
            }
            MegaCommands::FuseMount(path) => {
                tracing::info!("Mounting fuse at {:?}", path);
                let mut mp_lock = self.mount_point.write().await;
                *mp_lock = Some(path);
                self.mounted.store(true, Ordering::Relaxed);
            }
            MegaCommands::FuseUnmount => {
                tracing::info!("Unmounting fuse");
                let mut mp_lock = self.mount_point.write().await;
                *mp_lock = None;
                self.mounted.store(false, Ordering::Relaxed);
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
    ///
    /// # Warning
    ///
    /// DO NOT add any blocking code here.
    pub(crate) async fn init(&self) {
        if self.initialized.load(Ordering::Acquire) {
            tracing::error!("MegaCore is already initialized");
            return;
        } else {
            self.initialized.store(true, Ordering::Release);
        }
        vault::pgp::delete_keys().await;

        // Try to load pgp keys from vault.
        if let Some(pk) = vault::pgp::load_pub_key().await {
            let sk = vault::pgp::load_sec_key().await.unwrap();
            self.pgp.set((pk, sk)).unwrap();
            tracing::debug!("Loaded pgp keys from vault");
        }
    }

    /// Launch Mega Http(s) and SSH servers.
    async fn launch(
        &self,
        http_addr: Option<SocketAddr>,
        ssh_addr: Option<SocketAddr>,
    ) -> MonoBeanResult<()> {
        if self.is_core_running().await {
            let err = "Mega core is already running";
            tracing::error!(err);
            return Err(MonoBeanError::MegaCoreError(err.to_string()));
        }

        let inner = MegaContext::new((*self.config).clone()).await;
        inner
            .services
            .mono_storage
            .init_monorepo(&self.config.monorepo)
            .await;

        let http_ctx = inner.clone();
        *self.http_options.write().await = http_addr.map(HttpOptions::new).or(None);
        let http_opt = self.http_options.clone();
        tokio::spawn(async move {
            let opt = &*http_opt.read().await;
            match opt {
                Some(http_opt) => {
                    let _ = http_opt.run_server(http_ctx).await;
                }
                None => {
                    tracing::error!("Failed to start http server, http options is not initialized");
                }
            }
        });

        let ssh_ctx = inner.clone();
        let ssh_opt = ssh_addr.map(SshOptions::new).or(None);
        *self.ssh_options.write().await = ssh_opt;
        let ssh_opt = self.ssh_options.clone();
        tokio::spawn(async move {
            let opt = &*ssh_opt.read().await;
            match opt {
                Some(ssh_opt) => {
                    let _ = ssh_opt.run_server(ssh_ctx).await;
                }
                None => {
                    tracing::error!("Failed to start ssh server, ssh options is not initialized");
                }
            }
        });

        *self.running_context.write().await = Some(inner);
        Ok(())
    }

    async fn shutdown(&self) {
        if let Some(http_options) = &*self.http_options.read().await {
            http_options.shutdown_server();
        }
        if let Some(ssh_options) = &*self.ssh_options.read().await {
            ssh_options.shutdown_server();
        }
        *self.http_options.write().await = None;
        *self.http_options.write().await = None;
        *self.running_context.write().await = None;
    }

    pub async fn is_core_running(&self) -> bool {
        self.running_context.read().await.is_some()
    }
}
