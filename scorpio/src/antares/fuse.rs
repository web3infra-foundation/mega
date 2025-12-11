use std::{path::PathBuf, sync::Arc};

use tracing::{info, warn};
use libfuse_fs::{
    passthrough::{new_passthroughfs_layer, PassthroughArgs},
    unionfs::{config::Config, layer::Layer, OverlayFs},
};
use tokio::task::JoinHandle;

use crate::server::mount_filesystem;

/// Antares union-fs wrapper: dicfuse lower + passthrough upper/CL.
pub struct AntaresFuse {
    pub mountpoint: PathBuf,
    pub upper_dir: PathBuf,
    pub dic: Arc<crate::dicfuse::Dicfuse>,
    pub cl_dir: Option<PathBuf>,
    /// Background task running the FUSE session.
    fuse_task: Option<JoinHandle<()>>,
}
use libfuse_fs::passthrough::newlogfs::LoggingFileSystem;
impl AntaresFuse {
    /// Build directories for upper / optional CL layers.
    pub async fn new(
        mountpoint: PathBuf,
        dic: Arc<crate::dicfuse::Dicfuse>,
        upper_dir: PathBuf,
        cl_dir: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        if let Some(cl) = &cl_dir {
            std::fs::create_dir_all(cl)?;
        }
        std::fs::create_dir_all(&upper_dir)?;
        std::fs::create_dir_all(&mountpoint)?;

        Ok(Self {
            mountpoint,
            upper_dir,
            dic,
            cl_dir,
            fuse_task: None,
        })
    }

    /// Compose the union filesystem instance.
    pub async fn build_overlay(&self) -> std::io::Result<OverlayFs> {
        // Build lower layers: optional CL, then a passthrough over the upper dir as a fallback lower.
        let mut lower_layers: Vec<Arc<dyn Layer>> = Vec::new();
        if let Some(cl_dir) = &self.cl_dir {
            let cl_layer = new_passthroughfs_layer(PassthroughArgs {
                root_dir: cl_dir,
                mapping: None::<String>,
            })
            .await?;
            lower_layers.push(Arc::new(cl_layer) as Arc<dyn Layer>);
        }

        lower_layers.push(self.dic.clone() as Arc<dyn Layer>);

        // Upper layer mirrors upper_dir to keep writes separated from lower layers.
        let upper_layer: Arc<dyn Layer> = Arc::new(
            new_passthroughfs_layer(PassthroughArgs {
                root_dir: &self.upper_dir,
                mapping: None::<String>,
            })
            .await?,
        );

        // passthrough Upper  - readwrite file system over upper dir
        // passthrough CL  - readwrite file system over upper dir
        // dicfuse  - readonly file and dictionary from mega

        let cfg = Config {
            mountpoint: self.mountpoint.clone(),
            do_import: true,
            ..Default::default()
        };

        OverlayFs::new(Some(upper_layer), lower_layers, cfg, 1)
    }

    /// Mount the composed unionfs into the provided mountpoint, spawning a background task to run the FUSE session.
    pub async fn mount(&mut self) -> std::io::Result<()> {
        if self.fuse_task.is_some() {
            info!(
                "mount request ignored because {} is already mounted",
                self.mountpoint.display()
            );
            return Ok(());
        }

        let overlay = self.build_overlay().await?;
        let logfs = LoggingFileSystem::new(overlay);
        let handle = mount_filesystem(logfs, self.mountpoint.as_os_str()).await;

        // Spawn background task to run the FUSE session
        let fuse_task = tokio::spawn(async move {
            // This will block until unmount is called
            let _ = handle.await;
        });

        self.fuse_task = Some(fuse_task);

        // Poll the mountpoint until it becomes accessible (up to ~1s) to avoid race on slow machines.
        const RETRIES: usize = 10;
        for attempt in 0..RETRIES {
            if tokio::fs::read_dir(&self.mountpoint).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if attempt == RETRIES - 1 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!(
                        "mountpoint {} not ready after {} attempts",
                        self.mountpoint.display(),
                        RETRIES
                    ),
                ));
            }
        }

        Ok(())
    }

    /// Unmount the FUSE session if mounted.
    pub async fn unmount(&mut self) -> std::io::Result<()> {
        if let Some(task) = self.fuse_task.take() {
            // Unmount via fusermount
            let mount_path = self.mountpoint.to_string_lossy().to_string();
            let output = tokio::process::Command::new("fusermount")
                .arg("-u")
                .arg(&mount_path)
                .output()
                .await?;

            if !output.status.success() {
                return Err(std::io::Error::other(format!(
                    "fusermount failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            // Wait for the FUSE task to complete and surface panic if any.
            if let Err(e) = task.await {
                tracing::warn!(
                    "fuse task panicked during unmount of {}: {:?}",
                    mount_path,
                    e
                );
            }
        }

        // NOTE: directories (mountpoint/upper/cl) are not removed here to avoid
        // deleting caller-managed paths; cleanup should be handled by the caller.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AntaresFuse;
    use crate::{dicfuse::Dicfuse, util::config};
    use std::path::PathBuf;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    #[tokio::test]
    #[ignore] // Run with: sudo -E $(which cargo) test --lib antares::fuse::tests::test_simple_passthrough_mount -- --exact --ignored --nocapture
    async fn test_simple_passthrough_mount() {
        // Simplified test using only passthrough layers (no Dicfuse)
        use libfuse_fs::{
            passthrough::{new_passthroughfs_layer, PassthroughArgs},
            unionfs::{config::Config, OverlayFs},
        };
        use std::sync::Arc;

        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges");
            println!("Run with: sudo -E cargo test --lib antares::fuse::tests::test_simple_passthrough_mount -- --exact --ignored --nocapture");
            return;
        }

        let base = PathBuf::from("/tmp/antares_simple_test");
        let _ = std::fs::remove_dir_all(&base);

        let mount = base.join("mnt");
        let upper = base.join("upper");
        let lower1 = base.join("lower1");
        let lower2 = base.join("lower2");

        // Create directories and test files
        std::fs::create_dir_all(&mount).unwrap();
        std::fs::create_dir_all(&upper).unwrap();
        std::fs::create_dir_all(&lower1).unwrap();
        std::fs::create_dir_all(&lower2).unwrap();

        std::fs::write(lower1.join("file1.txt"), b"from lower1").unwrap();
        std::fs::write(lower2.join("file2.txt"), b"from lower2").unwrap();

        // Build overlay
        let lower1_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &lower1,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let lower2_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &lower2,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let upper_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &upper,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let cfg = Config {
            mountpoint: mount.clone(),
            do_import: true,
            ..Default::default()
        };

        let overlay = OverlayFs::new(
            Some(Arc::new(upper_layer)),
            vec![Arc::new(lower2_layer), Arc::new(lower1_layer)],
            cfg,
            1,
        )
        .unwrap();

        println!(
            "Mounting simple passthrough overlay at: {}",
            mount.display()
        );
        let handle = crate::server::mount_filesystem(overlay, mount.as_os_str()).await;

        // Spawn background task
        let fuse_task = tokio::spawn(async move {
            let _ = handle.await;
        });

        // Give it time to initialize
        sleep(Duration::from_millis(200)).await;

        println!("Mount successful!");
        println!("Mountpoint: {}", mount.display());
        println!("Try in another terminal: ls -la {}", mount.display());
        println!("You should see file1.txt and file2.txt");

        // Keep mounted for inspection
        sleep(Duration::from_secs(5)).await;

        // Unmount
        println!("Unmounting...");
        let output = tokio::process::Command::new("fusermount")
            .arg("-u")
            .arg(&mount)
            .output()
            .await
            .unwrap();

        if !output.status.success() {
            eprintln!(
                "fusermount failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let _ = fuse_task.await;
        println!("Unmount successful!");

        // cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore] // Run with: sudo -E $(which cargo) test --lib antares::fuse::tests::test_antares_mount -- --exact --ignored --nocapture
    async fn test_antares_mount() {
        // Only  LoggingFileSystem DEBUG
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive("libfuse_fs::passthrough::newlogfs=debug".parse().unwrap()),
            )
            .try_init();
        // Ignore "already initialized" error when running multiple tests
        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }
        // Check if we have necessary privileges
        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges for open_by_handle_at");
            println!("Run with: sudo -E cargo test --lib antares::fuse::tests::test_antares_mount -- --exact --ignored --nocapture");
            println!("Skipping test...");
            return;
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_test_mount_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let cl = base.join("cl");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        let dic = Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;
        let mut fuse = AntaresFuse::new(
            mount.clone(),
            std::sync::Arc::new(dic),
            upper.clone(),
            Some(cl.clone()),
        )
        .await
        .unwrap();

        // Actually mount the filesystem
        println!("Mounting Antares overlay at: {}", mount.display());
        fuse.mount().await.unwrap();

        // Verify mount succeeded
        assert!(mount.exists(), "mount directory should exist");
        assert!(upper.exists(), "upper directory should exist");
        assert!(
            std::fs::read_dir(&mount).is_ok(),
            "mountpoint should be accessible"
        );

        // Let it run for a bit to ensure stability
        sleep(Duration::from_secs(1)).await;

        // Unmount
        println!("Unmounting...");
        fuse.unmount().await.unwrap();
        println!("Unmount successful!");

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore = "manual test with infinite loop, requires privileged FUSE mount"]
    async fn creates_dirs_and_placeholder_overlay() {
        // Ignore "already initialized" error when running multiple tests
        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        // Check if we have necessary privileges
        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges");
            println!("Run with: sudo -E cargo test --lib antares::fuse::tests::creates_dirs_and_placeholder_overlay -- --exact --ignored --nocapture");
            println!("Skipping test...");
            return;
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_test_job1_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let cl = base.join("cl");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        let dic = Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;
        let mut fuse = AntaresFuse::new(
            mount.clone(),
            std::sync::Arc::new(dic),
            upper.clone(),
            Some(cl.clone()),
        )
        .await
        .unwrap();

        // Mount the overlay filesystem
        fuse.mount().await.unwrap();

        // Verify directories were created and mount is accessible
        assert!(mount.exists(), "mount directory should exist");
        assert!(upper.exists(), "upper directory should exist");
        assert!(cl.exists(), "cl directory should exist");
        assert!(
            std::fs::read_dir(&mount).is_ok(),
            "mountpoint should be accessible"
        );

        // Unmount
        fuse.unmount().await.unwrap();

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }
}
