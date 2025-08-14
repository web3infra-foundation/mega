use crate::application::Action;
use crate::core::servers::{HttpOptions, SshOptions};
use crate::core::CoreConfigChanged;
use crate::error::{MonoBeanError, MonoBeanResult};
use async_channel::{Receiver, Sender};
use ceres::api_service::import_api_service::ImportApiService;
use ceres::api_service::mono_api_service::MonoApiService;
use ceres::api_service::ApiHandler;
use ceres::protocol::repo::Repo;
use common::config::Config;
use common::model::P2pOptions;
use context::AppContext as MegaContext;
use mercury::internal::object::tree::Tree;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, OnceCell, RwLock};
use vault::integration::vault_core::VaultCore;
use vault::pgp::{SignedPublicKey, SignedSecretKey};
use libra::command::log::{get_reachable_commits};
use mercury::internal::object::commit::Commit;

pub struct MegaCore {
    config: Arc<RwLock<Config>>,
    running_context: Arc<RwLock<Option<MegaContext>>>,
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
    MegaStart(Option<SocketAddr>, Option<SocketAddr>, P2pOptions),
    MegaShutdown,
    MegaRestart(Option<SocketAddr>, Option<SocketAddr>, P2pOptions),
    CoreStatus(
        oneshot::Sender<(
            /* core_running: */ bool,
            /* pgp_initialized: */ bool,
        )>,
    ),
    SaveFileChange(PathBuf),
    LoadOrInitPgp {
        chan: oneshot::Sender<MonoBeanResult<()>>,
        user_name: String,
        user_email: String,
        passwd: Option<String>,
    },
    ApplyUserConfig(Vec<CoreConfigChanged>),
    LoadFileTree {
        chan: oneshot::Sender<MonoBeanResult<Tree>>,
        path: Option<PathBuf>,
    },
    LoadFileContent {
        chan: oneshot::Sender<MonoBeanResult<String>>,
        id: String,
    },
    // GetRepoUrl {
    //     chan: oneshot::Sender<MonoBeanResult<String>>,
    // }
    GetPathHistory{
        chan: oneshot::Sender<MonoBeanResult<Vec<Commit>>>,
        path: String,
    },
}

impl Debug for MegaCore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("MegaCore")
            .field("config", &self.config)
            .field("mounted", &self.mounted)
            .finish()
    }
}

impl MegaCore {
    pub fn new(sender: Sender<Action>, receiver: Receiver<MegaCommands>, config: Config) -> Self {
        Self {
            config: Arc::from(RwLock::new(config)),
            running_context: Default::default(),
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
            MegaCommands::MegaStart(http_addr, ssh_addr, p2p_opt) => {
                tracing::info!("Starting Mega Core");
                self.launch(http_addr, ssh_addr, p2p_opt).await.unwrap();
            }
            MegaCommands::MegaShutdown => {
                tracing::info!("Shutting down Mega Core");
                self.shutdown().await;
            }
            MegaCommands::MegaRestart(http_addr, ssh_addr, p2p_opt) => {
                tracing::info!("Restarting Mega Core");
                self.shutdown().await;
                self.launch(http_addr, ssh_addr, p2p_opt).await.unwrap();
            }
            MegaCommands::CoreStatus(sender) => {
                let core_running = self.is_core_running().await;
                let pgp_initialized = self.pgp.initialized();
                sender.send((core_running, pgp_initialized)).unwrap();
            }
            MegaCommands::SaveFileChange(path) => {
                tracing::info!("Saving file change at {:?}", path);
            }
            MegaCommands::LoadOrInitPgp {
                chan,
                user_name,
                user_email,
                passwd,
            } => {
                if self.pgp.initialized() {
                    chan.send(Err(MonoBeanError::ReinitError)).unwrap();
                    return;
                }

                let guard = self.running_context.read().await;
                let vault_core = if let Some(ctx) = guard.as_ref() {
                    &ctx.vault
                } else {
                    let err_msg = "Mega core is not running, failed to get vault core";
                    tracing::error!(err_msg);
                    return;
                };

                if let Some(pk) = vault_core.load_pub_key() {
                    let sk = vault_core.load_sec_key().await.unwrap();
                    chan.send(Ok(())).unwrap();
                    self.pgp.set((pk, sk)).unwrap();
                } else {
                    let uid = format!("{user_name} <{user_email}>");
                    let params = VaultCore::params(
                        vault::pgp::KeyType::Rsa(2048),
                        passwd.clone(),
                        uid.as_ref(),
                    );
                    let (pk, sk) = vault_core.gen_pgp_keypair(params, passwd);
                    vault_core.save_keys(pk.clone(), sk.clone());
                    chan.send(Ok(())).unwrap();
                    self.pgp.set((pk, sk)).unwrap();
                }
            }
            MegaCommands::ApplyUserConfig(update) => {
                self.merge_config(update).await;
            }
            MegaCommands::LoadFileTree { chan, path } => {
                let tree = self.load_tree(path).await;
                chan.send(tree).unwrap();
            }
            MegaCommands::LoadFileContent { chan, id: path } => {
                let content = self.load_blob(path).await;
                chan.send(content).unwrap();
            }
            MegaCommands::GetPathHistory{ chan, path} => {
                let commits = self.get_path_history(&path).await;
                chan.send(commits).unwrap();
            }
            // MegaCommands::GetRepoUrl => {
            //
            // }
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

        let guard = self.running_context.read().await;
        let vault_core = if let Some(ctx) = guard.as_ref() {
            &ctx.vault
        } else {
            let err_msg = "Mega core is not running, failed to get vault core";
            tracing::error!(err_msg);
            return;
        };

        // Try to load pgp keys from vault.
        if let Some(pk) = vault_core.load_pub_key() {
            let sk = vault_core.load_sec_key().await.unwrap();
            self.pgp.set((pk, sk)).unwrap();
            tracing::debug!("Loaded pgp keys from vault");
        }
    }

    /// Launch Mega Http(s) and SSH servers.
    async fn launch(
        &self,
        http_addr: Option<SocketAddr>,
        ssh_addr: Option<SocketAddr>,
        p2p_opt: P2pOptions,
    ) -> MonoBeanResult<()> {
        if self.is_core_running().await {
            let err = "Mega core is already running";
            tracing::error!(err);
            return Err(MonoBeanError::MegaCoreError(err.to_string()));
        }

        let config = self.config.read().await.clone();

        #[cfg(test)]
        let inner = MegaContext::mock(config.clone()).await;

        #[cfg(not(test))]
        let inner = MegaContext::new(config.clone()).await;

        let http_ctx = inner.clone();
        *self.http_options.write().await = http_addr
            .map(|addr| HttpOptions::new(addr, p2p_opt))
            .or(None);
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
        *self.ssh_options.write().await = None;
        *self.running_context.write().await = None;
    }

    /// choose ImportApiService or MonoApiService judging by path prefix
    async fn api_handler(&self, path: impl AsRef<Path>) -> MonoBeanResult<Box<dyn ApiHandler>> {
        let ctx = self.running_context.read().await.clone();
        if ctx.is_none() {
            let err_msg = "Mega core is not running";
            tracing::error!(err_msg);
            return Err(MonoBeanError::MegaCoreError(err_msg.to_string()));
        }

        let ctx = ctx.unwrap();
        let import_dir = ctx.config.monorepo.import_dir.clone();

        if path.as_ref().starts_with(&import_dir) && path.as_ref() != import_dir {
            if let Some(model) = ctx
                .storage
                .git_db_storage()
                .find_git_repo_like_path(path.as_ref().to_string_lossy().as_ref())
                .await
                .unwrap()
            {
                let repo: Repo = model.into();
                //tracing::debug!("@@@@@@@@@@@@@@@@@@@@@@@@Found repo {:?}", repo.repo_id);
                return Ok(Box::new(ImportApiService {
                    storage: ctx.storage.clone(),
                    repo,
                }));
            }
        }
        let ret: Box<dyn ApiHandler> = Box::new(MonoApiService {
            storage: ctx.storage.clone(),
        });

        // Rust-analyzer cannot infer the type of `ret` correctly and always reports an error.
        // Use `.into()` to workaround this issue.
        #[allow(clippy::useless_conversion)]
        Ok(ret.into())
    }

    async fn load_tree(&self, path: Option<PathBuf>) -> MonoBeanResult<Tree> {
        let path = path.unwrap_or(PathBuf::from("/"));
        let path = path
            .components()
            .filter(|c| !matches!(c, Component::RootDir))
            .fold("/".to_owned(), |acc, e| {
                acc + &e.as_os_str().to_string_lossy() + "/"
            });
        let path = PathBuf::from(path);
        tracing::debug!("!!!!!!!!!!!!!!!!Loading tree from path: {}", path.display());

        let handler = self.api_handler(&path).await?;
        let tree = handler.search_tree_by_path(&path).await;
        match tree {
            Ok(Some(tree)) => Ok(tree),
            _ => {
                let err_msg = format!("Failed to load tree: {path:?}");
                tracing::error!(err_msg);
                Err(MonoBeanError::MegaCoreError(err_msg))
            }
        }
    }

    async fn load_blob(&self, id: impl AsRef<str>) -> MonoBeanResult<String> {
        let ctx = self.running_context.read().await.clone().unwrap();
        let mono = MonoApiService {
            storage: ctx.storage,
        };
        let raw = mono
            .get_raw_blob_by_hash(id.as_ref())
            .await
            .map_err(|err| MonoBeanError::MegaCoreError(err.to_string()))?;
        match raw {
            Some(model) => match model.data {
                Some(data) => match String::from_utf8(data) {
                    Ok(string) => Ok(string),
                    Err(err) => {
                        let err_msg = format!("Invalid UTF-8 data: {err}");
                        tracing::error!(err_msg);
                        Err(MonoBeanError::MegaCoreError(err_msg))
                    }
                },
                None => {
                    let err_msg = "Blob data is missing".to_string();
                    tracing::error!(err_msg);
                    Err(MonoBeanError::MegaCoreError(err_msg))
                }
            },
            _ => Ok(String::default()),
        }
    }

    async fn merge_config(&self, update: Vec<CoreConfigChanged>) {
        let mut base = self.config.write().await;
        for entry in update {
            match entry {
                CoreConfigChanged::BaseDir(path) => base.base_dir = path,
                CoreConfigChanged::LogPath(path) => base.log.log_path = path,
                CoreConfigChanged::Level(level) => base.log.level = level,
                CoreConfigChanged::PrintStd(print_std) => base.log.print_std = print_std,
                CoreConfigChanged::DbType(db_type) => base.database.db_type = db_type,
                CoreConfigChanged::DbPath(db_path) => base.database.db_path = db_path,
                CoreConfigChanged::DbUrl(db_url) => base.database.db_url = db_url,
                CoreConfigChanged::MaxConnection(max_conn) => {
                    base.database.max_connection = max_conn
                }
                CoreConfigChanged::MinConnection(min_conn) => {
                    base.database.min_connection = min_conn
                }
                CoreConfigChanged::SqlxLogging(sqlx_logging) => {
                    base.database.sqlx_logging = sqlx_logging
                }
                CoreConfigChanged::ImportDir(dir) => base.monorepo.import_dir = dir,
                CoreConfigChanged::Admin(admin) => base.monorepo.admin = admin,
                CoreConfigChanged::RootDirs(dirs) => base.monorepo.root_dirs = dirs,
                CoreConfigChanged::EnableHttpAuth(enable) => {
                    base.authentication.enable_http_auth = enable
                }
                CoreConfigChanged::EnableTestUser(enable) => {
                    base.authentication.enable_test_user = enable
                }
                CoreConfigChanged::TestUserName(name) => base.authentication.test_user_name = name,
                CoreConfigChanged::TestUserToken(token) => {
                    base.authentication.test_user_token = token
                }
                CoreConfigChanged::PackDecodeMemSize(size) => base.pack.pack_decode_mem_size = size,
                CoreConfigChanged::PackDecodeDiskSize(size) => {
                    base.pack.pack_decode_disk_size = size
                }
                CoreConfigChanged::PackDecodeCachePath(path) => {
                    base.pack.pack_decode_cache_path = path
                }
                CoreConfigChanged::CleanCacheAfterDecode(clean) => {
                    base.pack.clean_cache_after_decode = clean
                }
                CoreConfigChanged::ChannelMessageSize(size) => {
                    base.pack.channel_message_size = size
                }
                CoreConfigChanged::LfsUrl(url) => base.lfs.ssh.http_url = url,
                CoreConfigChanged::GithubClientId(id) => {
                    if base.oauth.is_none() {
                        base.oauth = Some(common::config::OauthConfig::default());
                    }
                    if let Some(oauth) = &mut base.oauth {
                        oauth.github_client_id = id;
                    }
                }
                CoreConfigChanged::GithubClientSecret(secret) => {
                    if base.oauth.is_none() {
                        base.oauth = Some(common::config::OauthConfig::default());
                    }
                    if let Some(oauth) = &mut base.oauth {
                        oauth.github_client_secret = secret;
                    }
                }
                CoreConfigChanged::UiDomain(domain) => {
                    if base.oauth.is_none() {
                        base.oauth = Some(common::config::OauthConfig::default());
                    }
                    if let Some(oauth) = &mut base.oauth {
                        oauth.ui_domain = domain;
                    }
                }
                CoreConfigChanged::CookieDomain(domain) => {
                    if base.oauth.is_none() {
                        base.oauth = Some(common::config::OauthConfig::default());
                    }
                    if let Some(oauth) = &mut base.oauth {
                        oauth.cookie_domain = domain;
                    }
                }
            }
        }
    }

    pub async fn is_core_running(&self) -> bool {
        self.running_context.read().await.is_some()
    }

    async fn get_path_history(&self,path: impl AsRef<Path>) ->  MonoBeanResult<Vec<Commit>>{

        let api_handler = self.api_handler(&path).await?;
        let cur_commit = api_handler.get_latest_commit(path.as_ref().to_path_buf())
            .await
            .map_err(|e| MonoBeanError::MegaCoreError(format!("Failed to get latest commit: {:?}", e)))?;


         let mut reachable_commits = get_reachable_commits(cur_commit.oid.clone()).await;
        //let mut reachable_commits = get_reachable_commits("").await;
        reachable_commits.sort_by_key(|c| c.committer.timestamp);
        tracing::debug!("Reachable commits: {:?}", reachable_commits);



        Ok(reachable_commits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_mega_resource;
    use crate::config::APP_NAME;
    use crate::config::MEGA_CONFIG_PATH;
    use async_channel::bounded;
    use common::config::LogConfig;
    use common::config::{AuthConfig, BuildConfig,DbConfig, LFSConfig, MonoConfig, PackConfig};
    use gtk::gio;
    use gtk::glib;

    use std::fs;
    use std::net::{IpAddr, Ipv4Addr};

    use tempfile::TempDir;

    #[allow(dead_code)]
    async fn test_core(temp_base: &TempDir) -> MegaCore {
        let (tx, _) = bounded(1);
        let (_, cmd_rx) = bounded(1);
        let config = Config {
            base_dir: temp_base.path().to_path_buf(),
            log: LogConfig {
                log_path: temp_base.path().to_path_buf(),
                level: "debug".to_string(),
                print_std: true,
            },
            database: DbConfig {
                db_type: "sqlite".to_string(),
                db_path: temp_base.path().to_path_buf().join("test.db"),
                ..Default::default()
            },
            monorepo: MonoConfig::default(),
            pack: PackConfig::default(),
            authentication: AuthConfig::default(),
            lfs: LFSConfig::default(),
            oauth: None,
            build: BuildConfig::default(),
        };

        // make config saved in temp dir
        let config_dir = temp_base.path().join("etc");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");
        let toml_str = toml::to_string(&config).unwrap();
        fs::write(&config_path, toml_str).unwrap();
        let config = Config::new(config_path.to_str().unwrap()).unwrap();

        let core = MegaCore::new(tx, cmd_rx, config);
        core.init().await;
        core
    }

    #[tokio::test]
    async fn test_load_config() {
        // This unit test should always pass for now.
        // Later we will use a bit more complex mechanism to load config,
        // and this test will be able to detect if the loading mechanism is broken.
        // TODO: use `Config::load_sources` to load glib shcema
        if let Some(cargo_dir) = std::option_env!("CARGO_MANIFEST_DIR") {
            std::env::set_current_dir(cargo_dir).expect("Failed to set workspace dir");
        }
        let resources =
            gio::Resource::load("Monobean.gresource").expect("Failed to load resources");
        gio::resources_register(&resources);
        glib::set_application_name(APP_NAME);

        let bytes = load_mega_resource(MEGA_CONFIG_PATH);
        let content = String::from_utf8(bytes).expect("Mega core setting must be in utf-8");
        let _ = Config::load_str(content.as_str()).expect("Failed to parse mega core settings");
    }

    #[tokio::test]
    async fn test_launch_http() {
        let temp_base = TempDir::new().unwrap();

        let core = test_core(&temp_base).await;

        core.process_command(MegaCommands::MegaStart(
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080)),
            None,
            P2pOptions::default(),
        ))
        .await;
        assert!(core.http_options.read().await.is_some());
        assert!(!core.ssh_options.read().await.is_some());

        core.process_command(MegaCommands::MegaShutdown).await;
        assert!(core.http_options.read().await.is_none());
        assert!(core.ssh_options.read().await.is_none());
    }

    #[tokio::test]
    async fn test_launch_ssh() {
        let temp_base = TempDir::new().unwrap();

        let core = test_core(&temp_base).await;
        core.process_command(MegaCommands::MegaStart(
            None,
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 2222)),
            P2pOptions::default(),
        ))
        .await;
        assert!(core.http_options.read().await.is_none());
        assert!(core.ssh_options.read().await.is_some());

        core.process_command(MegaCommands::MegaShutdown).await;
        assert!(core.http_options.read().await.is_none());
        assert!(core.ssh_options.read().await.is_none());
    }

    #[tokio::test]
    async fn test_run_with_config() {}

    #[tokio::test]
    async fn test_get_history() {

    }

}
