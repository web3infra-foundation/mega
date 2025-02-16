use std::net::{IpAddr, SocketAddr};
use std::cell::RefCell;
use tokio::sync::{mpsc, Mutex};
use jupiter::context::Context as MegaContext;
use std::sync::Arc;
use mono::git_protocol::ssh::SshServer;
use std::collections::HashMap;
use bytes::BytesMut;
use axum_server::tls_rustls::RustlsConfig;
use russh::server::Server;
use mono::server::https_server::app;
use crate::config::{MEGA_HTTPS_CERT, MEGA_HTTPS_KEY};
use crate::error::MonoBeanResult;

#[derive(Debug, Clone)]
pub struct SshOptions {
    addr: SocketAddr,
    abort: RefCell<Option<mpsc::Sender<()>>>,
}

#[derive(Debug, Clone)]
pub struct HttpOptions {
    addr: SocketAddr,
    handle: axum_server::Handle,
}

impl HttpOptions {
    pub fn new(addr: SocketAddr) -> Self {
        let handle = axum_server::Handle::default();
        Self { addr, handle }
    }

    pub async fn run_server(&self, mega_ctx: MegaContext) -> MonoBeanResult<()> {
        let app = app(mega_ctx, self.addr.ip().to_string(), self.addr.port()).await;
        let cert = crate::core::load_mega_resource(MEGA_HTTPS_CERT);
        let key = crate::core::load_mega_resource(MEGA_HTTPS_KEY);
        let tls_config = RustlsConfig::from_pem(cert, key).await;

        if let Ok(tls_config) = tls_config {
            axum_server::bind_rustls(self.addr, tls_config)
                .handle(self.handle.clone())
                .serve(app.into_make_service())
                .await?;
        } else {
            tracing::warn!("Failed to load tls config, falling back to HTTP server...");
            axum_server::bind(self.addr)
                .handle(self.handle.clone())
                .serve(app.into_make_service())
                .await?;
        }
        Ok(())
    }

    pub fn shutdown_server(&self) {
        tracing::warn!("HTTP server is shutting down...");
        self.handle.shutdown();
    }
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self::new(SocketAddr::new(IpAddr::V4([0, 0, 0, 0].into()), 8080))
    }
}

impl SshOptions {
    pub fn new(addr: SocketAddr) -> Self {
        let abort = RefCell::new(None);
        Self { addr, abort }
    }

    pub async fn run_server(&self, mega_ctx: MegaContext) -> MonoBeanResult<()> {
        // Use rusty vault configurations...
        let (tx, mut rx) = mpsc::channel::<()>(1);
        let key = mono::server::ssh_server::load_key();
        let ssh_config = russh::server::Config {
            auth_rejection_time: std::time::Duration::from_secs(3),
            keys: vec![key],
            preferred: Default::default(),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            ..Default::default()
        };
        let ssh_config = Arc::new(ssh_config);
        let mut ssh_server = SshServer {
            clients: Arc::new(Mutex::new(HashMap::new())),
            id: 0,
            context: mega_ctx,
            smart_protocol: None,
            data_combined: BytesMut::new(),
        };

        loop {
            tokio::select! {
                _ = rx.recv() => {
                    tracing::info!("SSH server is shutting down...");
                    break;
                }
                _ = ssh_server.run_on_address(ssh_config.clone(), self.addr) => {}
            }
        }
        Ok(())
    }

    pub fn shutdown_server(&self) {
        if self.abort.borrow().is_some() {
            let abort = self.abort.borrow_mut().take().unwrap();
            if let Ok(_) = abort.try_send(()) {
                return;
            }
        }
        tracing::warn!("SSH server is not running, aborting...");
    }
}

impl Default for SshOptions {
    fn default() -> Self {
        Self::new(SocketAddr::new(IpAddr::V4([0, 0, 0, 0].into()), 2222))
    }
}