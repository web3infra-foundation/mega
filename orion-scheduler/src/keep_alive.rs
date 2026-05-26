use std::sync::Arc;

use anyhow::Result;
use qlean::{ImageConfig, Machine, MachineConfig};
use tokio::sync::Mutex;

/// Wrapper around Machine that keeps it alive after initial operations.
/// This allows multiple operations (deploy, start_orion, etc.) on the same VM.
pub struct KeepAliveMachine {
    machine: Arc<Mutex<Option<Machine>>>,
}

impl KeepAliveMachine {
    /// Create a new VM and keep it alive.
    ///
    /// `image_config` can be:
    /// - `Some(ImageConfig)` with a local `path` + `digest` — uses custom image
    /// - `Some(ImageConfig)` with a remote `url` + `digest` — qlean downloads it
    /// - `None` — uses qlean's built-in Debian image
    pub async fn new(
        vm_name: &str,
        image_config: Option<ImageConfig>,
        disk_gb: Option<u32>,
        cpus: Option<u32>,
        memory_mb: Option<u32>,
    ) -> Result<Self> {
        tracing::info!("[keep-alive] Creating VM: {}", vm_name);

        let config = MachineConfig {
            core: cpus.unwrap_or(4),
            mem: memory_mb.unwrap_or(8192),
            disk: disk_gb,
            ..Default::default()
        };
        tracing::info!(
            "[keep-alive] VM config: cpus={}, memory_mb={}",
            config.core,
            config.mem
        );

        let image = if let Some(cfg) = image_config {
            if cfg.source.is_some() {
                tracing::info!("[keep-alive] Using custom image (source set)");
            } else {
                tracing::info!("[keep-alive] Using default Debian image (no custom config)");
            }
            qlean::Image::new(cfg).await?
        } else {
            tracing::info!("[keep-alive] Using default Debian image");
            qlean::Image::new(
                ImageConfig::default()
                    .with_distro(qlean::Distro::Debian)
                    .with_arch(qlean::GuestArch::Amd64),
            )
            .await?
        };

        let mut machine = Machine::new(&image, &config).await?;
        machine.init().await?;

        tracing::info!("[keep-alive] VM {} initialized and running", vm_name);

        Ok(Self {
            machine: Arc::new(Mutex::new(Some(machine))),
        })
    }

    /// Execute a command in the VM
    pub async fn exec(&self, cmd: &str) -> Result<std::process::Output> {
        let mut guard = self.machine.lock().await;
        if let Some(machine) = guard.as_mut() {
            tracing::info!("[keep-alive] Executing: {}", cmd);
            let output = machine.exec(cmd).await?;
            Ok(output)
        } else {
            anyhow::bail!("VM has been shut down")
        }
    }

    /// Upload a file to the VM
    pub async fn upload(
        &self,
        local: impl AsRef<std::path::Path>,
        remote: impl AsRef<std::path::Path>,
    ) -> Result<()> {
        let mut guard = self.machine.lock().await;
        if let Some(machine) = guard.as_mut() {
            let local_path = local.as_ref();
            let remote_path_str = remote.as_ref().to_string_lossy().into_owned();
            tracing::info!(
                "[keep-alive] Uploading: {} -> {}",
                local_path.display(),
                remote_path_str
            );
            machine.upload(local, remote).await?;
            Ok(())
        } else {
            anyhow::bail!("VM has been shut down")
        }
    }

    /// Shutdown the VM
    pub async fn shutdown(self) -> Result<()> {
        tracing::info!("[keep-alive] Shutting down VM");
        let mut guard = self.machine.lock().await;
        if let Some(mut machine) = guard.take() {
            machine.shutdown().await?;
            tracing::info!("[keep-alive] VM shutdown complete");
        }
        Ok(())
    }

    /// Get the VM's IP address by running hostname -I inside the VM
    pub async fn get_ip(&self) -> Result<Option<String>> {
        let output = self.exec("hostname -I | awk '{print $1}'").await?;
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if ip.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ip))
        }
    }
}

impl Clone for KeepAliveMachine {
    fn clone(&self) -> Self {
        Self {
            machine: Arc::clone(&self.machine),
        }
    }
}
