use crate::application::{runtime, Action};
use crate::config::{MEGA_CONFIG_PATH, MEGA_HTTPS_CERT, MEGA_HTTPS_KEY};
use adw::gio;
use adw::gio::ResourceLookupFlags;
use adw::prelude::InputStreamExtManual;
use async_channel::{Receiver, Sender};
use common::config::{Config, LogConfig};
use jupiter::context::{Context as MegaContext, Context};
use mono::server::https_server::app;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use axum::ServiceExt;
use axum_server::tls_rustls::RustlsConfig;
use russh::Preferred;
use tokio::task::{futures, JoinHandle};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use bytes::BytesMut;
use russh::server::Server;
use tokio::sync::Mutex;
use mono::git_protocol::ssh::SshServer;
use crate::error::{MonoBeanError, MonoBeanResult};

#[derive(Debug)]
struct SshOptions {
    addr: SocketAddr,
}

#[derive(Debug)]
struct HttpOptions {
    addr: SocketAddr,
}

pub struct MegaCore {
    config: Config,
    running_context: Option<MegaContext>,
    mount_point: Option<PathBuf>,

    ssh_options: Option<SshOptions>,
    http_options: Option<HttpOptions>,
    mounted: bool,

    sender: Sender<Action>,
}

impl Debug for MegaCore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("MegaCore")
            .field("config", &self.config)
            .field("mount_point", &self.mount_point)
            .field("ssh_options", &self.ssh_options)
            .field("http_options", &self.http_options)
            .field("mounted", &self.mounted)
            .finish()
    }
}

impl MegaCore {
    pub fn new(sender: Sender<Action>) -> Self {
        let bytes = Self::load_mega_resource(MEGA_CONFIG_PATH);
        let content = String::from_utf8(bytes).expect("Mega core setting must be in utf-8");
        let config =
            Config::load_str(content.as_str()).expect("Failed to parse mega core settings");

        init_mega_log(&config.log);

        Self {
            config,
            mount_point: None,
            ssh_options: None,
            http_options: None,
            running_context: Default::default(),

            mounted: false,
            sender,
        }
    }

    fn load_mega_resource(path: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        let _ = gio::resources_open_stream(path, ResourceLookupFlags::all())
            .expect("Failed to load mega core settings")
            .read_all(&mut bytes, gio::Cancellable::NONE)
            .expect("Failed to read mega core settings");
        bytes
    }

    /// Entry point of Mega Core.
    /// ## Warning:
    /// This function must be called in a tokio runtime.
    pub async fn launch(&mut self) -> MonoBeanResult<()> {
        let context = Context::new(self.config.clone()).await;
        context
            .services
            .mono_storage
            .init_monorepo(&self.config.monorepo)
            .await;

        self.running_context = Some(context);

        let (http_res, ssh_res) = tokio::join!(
            Self::serve_http(self.running_context.clone().unwrap(), self.http_options.take()),
            Self::serve_ssh(self.running_context.clone().unwrap(), self.ssh_options.take()),
        );

        let _ = http_res.map_err(|e| MonoBeanError::MegaCoreError(format!("Failed to serve http: {}", e.to_string())))?;
        let _ = ssh_res.map_err(|e| MonoBeanError::MegaCoreError(format!("Failed to serve ssh: {}", e.to_string())))?;
        Ok(())
    }

    pub async fn shutdown(&self) {
        unimplemented!()
    }

    async fn serve_http(mega_ctx: MegaContext, http_options: Option<HttpOptions>) -> MonoBeanResult<()> {
        if let Some(opt) = http_options {
            let app = app(mega_ctx, opt.addr.ip().to_string(), opt.addr.port()).await;
            let cert = Self::load_mega_resource(MEGA_HTTPS_CERT);
            let key = Self::load_mega_resource(MEGA_HTTPS_KEY);
            let tls_config = RustlsConfig::from_pem(cert, key).await;

            if let Ok(tls_config) = tls_config {
                axum_server::bind_rustls(opt.addr, tls_config).serve(app.into_make_service()).await?;
            } else {
                tracing::warn!("Failed to load tls config, falling back to HTTP server...");
                axum_server::bind(opt.addr).serve(app.into_make_service()).await?;
            }
        }
        Ok(())
    }

    async fn serve_ssh(mega_ctx: MegaContext, ssh_options: Option<SshOptions>) -> MonoBeanResult<()> {
        if let Some(opt) = ssh_options {
            // Use rusty vault configurations...
            let key = mono::server::ssh_server::load_key();
            let ssh_config = russh::server::Config {
                auth_rejection_time: std::time::Duration::from_secs(3),
                keys: vec![key],
                preferred: Default::default(),
                auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
                ..Default::default()
            };
            let mut ssh_server = SshServer {
                // TODO: Change this to DashMap
                clients: Arc::new(Mutex::new(HashMap::new())),
                id: 0,
                context: mega_ctx,
                smart_protocol: None,
                data_combined: BytesMut::new(),
            };
            ssh_server.run_on_address(Arc::from(ssh_config), opt.addr).await?;
        }

        Ok(())
    }
}

// TODO: move to `application.rs` to globally initialize the log
fn init_mega_log(config: &LogConfig) {
    let log_level = match config.level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let file_appender = tracing_appender::rolling::hourly(config.log_path.clone(), "monobean-logs");

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
