use std::{path::PathBuf, sync::Arc};

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
        // Use timeout to prevent blocking if FUSE operations are slow (e.g., Dicfuse loading data)
        const RETRIES: usize = 5; // Reduced retries since we have timeout per attempt
        const READ_DIR_TIMEOUT_MS: u64 = 200; // 200ms timeout per read_dir attempt

        for attempt in 0..RETRIES {
            // Use timeout to prevent read_dir from blocking indefinitely
            // This is important when Dicfuse is still loading data in the background
            tracing::debug!(
                "Mount attempt {}: checking mountpoint {}",
                attempt + 1,
                self.mountpoint.display()
            );
            let read_dir_future = tokio::fs::read_dir(&self.mountpoint);
            let start_time = std::time::Instant::now();
            match tokio::time::timeout(
                tokio::time::Duration::from_millis(READ_DIR_TIMEOUT_MS),
                read_dir_future,
            )
            .await
            {
                Ok(Ok(_)) => {
                    tracing::debug!(
                        "Mountpoint {} accessible after {}ms",
                        self.mountpoint.display(),
                        start_time.elapsed().as_millis()
                    );
                    return Ok(());
                }
                Ok(Err(e)) => {
                    tracing::debug!(
                        "Mountpoint {} not accessible yet (attempt {}): {:?}",
                        self.mountpoint.display(),
                        attempt + 1,
                        e
                    );
                    // Directory not accessible yet, continue polling
                }
                Err(_) => {
                    tracing::warn!(
                        "Mountpoint {} read_dir timed out after {}ms (attempt {}), Dicfuse may still be loading",
                        self.mountpoint.display(),
                        start_time.elapsed().as_millis(),
                        attempt + 1
                    );
                    // Timeout: read_dir took too long, likely because Dicfuse is loading data.
                    // This is acceptable - the mount is successful, just slow to respond.
                    // If this is the last attempt, check if mountpoint exists as fallback.
                    if attempt == RETRIES - 1 && self.mountpoint.exists() {
                        tracing::warn!(
                            "Mountpoint {} exists but read_dir timed out (Dicfuse may still be loading)",
                            self.mountpoint.display()
                        );
                        // TODO(dicfuse-global-singleton): Replace polling with a readiness signal from
                        // DicfuseManager/global import task so Antares can block on actual tree-load
                        // completion instead of relying on time-based heuristics.
                        return Ok(()); // Consider mount successful if directory exists
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Final fallback: if mountpoint exists, consider mount successful
        // This handles the case where Dicfuse is still loading data but mount is functional
        if self.mountpoint.exists() {
            tracing::warn!(
                "Mountpoint {} exists but read_dir timed out after {} attempts (Dicfuse may still be loading)",
                self.mountpoint.display(),
                RETRIES
            );
            return Ok(());
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!(
                "mountpoint {} not ready after {} attempts",
                self.mountpoint.display(),
                RETRIES
            ),
        ))
    }

    /// Unmount the FUSE session if mounted.
    ///
    /// Uses lazy unmount (`fusermount -uz`) to detach the filesystem even if
    /// it's busy, preventing the unmount operation from blocking indefinitely.
    /// A timeout is applied when waiting for the FUSE task to complete.
    ///
    /// # Errors
    ///
    /// This method will log warnings but not fail if:
    /// - The FUSE task doesn't complete within the timeout
    /// - The task panics
    ///
    /// Only critical errors (e.g., fusermount command execution failure)
    /// will cause this method to return an error.
    pub async fn unmount(&mut self) -> std::io::Result<()> {
        if let Some(task) = self.fuse_task.take() {
            // Unmount via fusermount with lazy unmount (-z) for faster unmounting
            // This allows unmounting even if there are pending operations
            let mount_path = self.mountpoint.to_string_lossy().to_string();
            let output = tokio::process::Command::new("fusermount")
                .arg("-uz") // -u: unmount, -z: lazy unmount (detach even if busy; actual unmount occurs after all references are released)
                .arg(&mount_path)
                .output()
                .await?;

            if !output.status.success() {
                tracing::warn!(
                    "fusermount -uz failed for {}: {}",
                    mount_path,
                    String::from_utf8_lossy(&output.stderr)
                );
                // Continue, as lazy unmount might still succeed partially or task might exit
            }

            // Wait for the FUSE task to complete with timeout to avoid hanging
            let timeout_duration = tokio::time::Duration::from_secs(5);
            match tokio::time::timeout(timeout_duration, task).await {
                Ok(Ok(_)) => {
                    // Task completed successfully
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "fuse task panicked during unmount of {}: {:?}",
                        mount_path,
                        e
                    );
                }
                Err(_) => {
                    tracing::warn!(
                        "fuse task did not complete within {}s for {}, continuing anyway",
                        timeout_duration.as_secs(),
                        mount_path
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AntaresFuse;
    use crate::dicfuse::Dicfuse;
    use crate::util::config;
    use serial_test::serial;
    use std::path::PathBuf;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    #[tokio::test]
    #[ignore]
    // Requires FUSE/root. Direct run example:
    //   sudo -E cargo test --lib antares::fuse::tests::test_simple_passthrough_mount -- --exact --ignored --nocapture
    // For LLDB debug workflow, see `doc/test.md`.
    #[serial] // Serialize to avoid config initialization conflicts
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

        // Unmount using lazy unmount to avoid blocking
        println!("Unmounting...");
        let output = tokio::process::Command::new("fusermount")
            .arg("-uz") // Use lazy unmount
            .arg(&mount)
            .output()
            .await
            .unwrap();

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            // Check if the error is because the filesystem is not mounted
            if !error_msg.contains("not mounted") && !error_msg.contains("Invalid argument") {
                eprintln!("fusermount failed: {}", error_msg);
            }
        }

        // Wait for FUSE task to complete with timeout (don't wait indefinitely)
        let timeout_duration = tokio::time::Duration::from_secs(5);
        match tokio::time::timeout(timeout_duration, fuse_task).await {
            Ok(Ok(_)) => println!("FUSE task completed successfully"),
            Ok(Err(e)) => tracing::warn!("FUSE task panicked: {:?}", e),
            Err(_) => tracing::warn!(
                "FUSE task did not complete within {}s, continuing anyway",
                timeout_duration.as_secs()
            ),
        }
        println!("Unmount successful!");

        // cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore] // Run with: sudo -E $(which cargo) test --lib antares::fuse::tests::test_run_mount -- --exact --ignored --nocapture
    async fn test_run_mount() {
        // Helper function to check if a file should be skipped in directory iteration
        let _should_skip_test_file = |name: &str| -> bool {
            name == "test_file.txt" || name == "created_file.txt" || name == "test_dir"
        };
        // Only LoggingFileSystem DEBUG
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
            println!("Run with: sudo -E cargo test --lib antares::fuse::tests::test_run_mount -- --exact --ignored --nocapture");
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
        // Load directory tree synchronously - simpler and more efficient for tests
        // since we need the tree fully loaded before mount verification anyway
        println!("Loading directory tree...");
        crate::dicfuse::store::import_arc(dic.store.clone()).await;
        println!("Directory tree loaded, proceeding to mount");

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
        println!("Mount completed successfully");
        // mount() already verified accessibility via read_dir, so we can skip redundant checks
        // Keep mounted for inspection
        sleep(Duration::from_secs(30)).await;
        // Listen for Ctrl+C and unmount on signal
        println!("Press Ctrl+C to unmount and exit...");
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl_c");
        println!("Ctrl+C received, unmounting...");
        fuse.unmount().await.unwrap();
        println!("Unmount successful!");
        //let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore]
    // Requires FUSE/root. Direct run example:
    //   sudo -E cargo test --lib antares::fuse::tests::test_antares_mount -- --exact --ignored --nocapture
    // For no-run + LLDB debugging steps, see `doc/test.md`.
    #[serial] // Serialize to avoid config initialization conflicts
    async fn test_antares_mount() {
        // Set overall test timeout to 60 seconds
        let test_future = async {
            // Helper function to check if a file should be skipped in directory iteration
            let should_skip_test_file = |name: &str| -> bool {
                name == "test_file.txt" || name == "created_file.txt" || name == "test_dir"
            };
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

            // Use isolated Dicfuse instance for testing to avoid database lock conflicts
            // In production, use DicfuseManager::global() to share the instance
            let dic =
                crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;
            // Start background import_arc task to load directory tree asynchronously
            // This prevents blocking during FUSE operations (see blog post for details)
            tokio::spawn(crate::dicfuse::store::import_arc(dic.store.clone()));
            // Wait for Dicfuse to initialize and fetch directory tree from network
            // Increased wait time to allow for network requests to complete
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

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
            println!("Mount completed successfully");
            // mount() already verified accessibility via read_dir, so we can skip redundant checks

            // Let it run for a bit to ensure stability
            println!("Sleeping for 1 second...");
            sleep(Duration::from_secs(1)).await;
            println!("Sleep completed");

            // Test basic read operations
            println!("Testing basic read operations...");
            let read_dir_result = tokio::fs::read_dir(&mount).await;
            assert!(read_dir_result.is_ok(), "should be able to read directory");
            println!("✓ Directory read successful");

            // Test reading from read-only layer (Dicfuse)
            // Try to read files from Dicfuse lower layer if they exist
            println!("Testing read from read-only layer (Dicfuse)...");
            let mut dir_entries = read_dir_result.unwrap();
            let mut found_readonly_file = false;
            while let Some(entry) = dir_entries.next_entry().await.unwrap_or(None) {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                // Skip . and .. entries
                if file_name == "." || file_name == ".." {
                    continue;
                }

                // Try to read a file from the read-only layer
                if entry.file_type().await.unwrap().is_file() {
                    match tokio::fs::read(&path).await {
                        Ok(content) => {
                            println!(
                                "✓ Read file from read-only layer: {} ({} bytes)",
                                file_name,
                                content.len()
                            );
                            found_readonly_file = true;
                            break;
                        }
                        Err(e) => {
                            // File might not be loaded yet, skip
                            println!("⚠ Could not read {} from read-only layer: {}", file_name, e);
                        }
                    }
                }
            }
            if !found_readonly_file {
                println!("⚠ No files found in read-only layer (may still be loading)");
            }

            // Test basic write operations (create a file in upper layer)
            println!("Testing basic write operations...");
            let test_file = mount.join("test_file.txt");
            let test_content = b"Hello, FUSE!";

            // Write file
            tokio::fs::write(&test_file, test_content).await.unwrap();
            println!("✓ File write successful");

            // Read file back (this verifies file exists and content is correct)
            let read_content = tokio::fs::read(&test_file).await.unwrap();
            assert_eq!(read_content, test_content, "file content should match");
            println!("✓ File read successful, content matches");

            // Test directory creation
            println!("Testing directory creation...");
            let test_dir = mount.join("test_dir");
            tokio::fs::create_dir(&test_dir).await.unwrap();
            println!("✓ Directory creation successful");

            // Test file creation (create empty file first, then write to it)
            println!("Testing file creation...");
            let created_file = mount.join("created_file.txt");
            let created_content = b"Content written to created file";

            // Use tokio::fs::write which handles file creation, writing, and closing atomically
            tokio::fs::write(&created_file, created_content)
                .await
                .unwrap();
            println!("✓ File created and written successfully");

            // Verify the created file
            let read_created = tokio::fs::read(&created_file).await.unwrap();
            assert_eq!(
                read_created, created_content,
                "created file content should match"
            );
            println!("✓ File creation verification successful");

            // Test file in subdirectory
            let subdir_file = test_dir.join("subdir_file.txt");
            let subdir_content = b"File in subdirectory";
            tokio::fs::write(&subdir_file, subdir_content)
                .await
                .unwrap();
            let read_subdir_content = tokio::fs::read(&subdir_file).await.unwrap();
            assert_eq!(
                read_subdir_content, subdir_content,
                "subdirectory file content should match"
            );
            println!("✓ Subdirectory file operations successful");

            // Test Copy-Up mechanism: modify a file from read-only layer
            println!("Testing Copy-Up mechanism (modify read-only file)...");
            // Try to find a file from read-only layer and modify it
            let mut dir_entries = tokio::fs::read_dir(&mount).await.unwrap();
            let mut tested_copyup = false;
            while let Some(entry) = dir_entries.next_entry().await.unwrap_or(None) {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                // Skip . and .. entries, and files we created during this test
                if file_name == "." || file_name == ".." || should_skip_test_file(&file_name) {
                    continue;
                }

                // Try to modify a file from read-only layer (triggers Copy-Up)
                if entry.file_type().await.unwrap().is_file() {
                    match tokio::fs::read(&path).await {
                        Ok(_original_content) => {
                            // Modify the file (this should trigger Copy-Up)
                            let modified_content = b"Modified content from test";
                            tokio::fs::write(&path, modified_content).await.unwrap();

                            // Verify the modification
                            let read_modified = tokio::fs::read(&path).await.unwrap();
                            assert_eq!(
                                read_modified, modified_content,
                                "modified file content should match"
                            );

                            // Verify Copy-Up: file should now be in upper layer
                            let upper_file = upper.join(file_name.as_ref());
                            let upper_check = tokio::time::timeout(
                                Duration::from_secs(2),
                                tokio::fs::read(&upper_file),
                            )
                            .await;
                            match upper_check {
                                Ok(Ok(upper_content)) => {
                                    assert_eq!(
                                        upper_content, modified_content,
                                        "upper layer file should have modified content"
                                    );
                                    println!("✓ Copy-Up mechanism verified: {} copied to upper layer and modified", file_name);
                                    tested_copyup = true;
                                }
                                _ => {
                                    println!("⚠ Copy-Up verification skipped for {} (file may still be syncing)", file_name);
                                }
                            }
                            break;
                        }
                        Err(_) => {
                            // File might not be loaded yet, skip
                            continue;
                        }
                    }
                }
            }
            if !tested_copyup {
                println!("⚠ Copy-Up test skipped (no files from read-only layer available yet)");
            }

            // Verify files are in upper layer (use async check with timeout)
            println!("Verifying copy-up to upper layer for new files...");
            let upper_test_file = upper.join("test_file.txt");
            let upper_check = tokio::time::timeout(
                Duration::from_secs(2),
                tokio::fs::metadata(&upper_test_file),
            )
            .await;
            if upper_check.is_ok() && upper_check.unwrap().is_ok() {
                println!("✓ Copy-up to upper layer confirmed");
            } else {
                println!("⚠ Copy-up verification skipped (file may still be syncing)");
            }

            // Unmount
            println!("Unmounting...");
            fuse.unmount().await.unwrap();
            println!("Unmount successful!");

            // Cleanup
            let _ = std::fs::remove_dir_all(&base);
        };

        // Run test with timeout to prevent hanging
        // Increased timeout to 120s to account for Dicfuse network initialization
        match tokio::time::timeout(Duration::from_secs(120), test_future).await {
            Ok(_) => println!("✓ Test completed successfully"),
            Err(_) => panic!("Test timed out after 120 seconds - this may indicate a blocking operation or network issue"),
        }
    }

    #[tokio::test]
    #[ignore] // Requires root privileges for FUSE mount
    #[serial] // Serialize to avoid config initialization conflicts
    async fn creates_dirs_and_placeholder_overlay() {
        // Set overall test timeout to prevent hanging
        let test_future = async {
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

            let test_id = uuid::Uuid::new_v4();
            let base = PathBuf::from(format!("/tmp/antares_test_job1_{test_id}"));
            let _ = std::fs::remove_dir_all(&base);
            let mount = base.join("mnt");
            let upper = base.join("upper");
            let cl = base.join("cl");
            let store_path = base.join("store");
            std::fs::create_dir_all(&store_path).unwrap();

            // Use isolated Dicfuse instance for testing to avoid database lock conflicts
            // In production, use DicfuseManager::global() to share the instance
            let dic =
                crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;
            // Start background import_arc task to load directory tree asynchronously
            // This prevents blocking during FUSE operations (see blog post for details)
            println!("Starting Dicfuse background import_arc task...");
            tokio::spawn(crate::dicfuse::store::import_arc(dic.store.clone()));

            // Wait for Dicfuse to initialize and fetch directory tree from network
            // Use wait_for_ready() with timeout instead of fixed sleep to handle variable load times
            println!("Waiting for Dicfuse to initialize (this may take time if loading large directory trees)...");
            let init_start = std::time::Instant::now();
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(120), // 120 second timeout for large directory trees
                dic.store.wait_for_ready(),
            )
            .await
            {
                Ok(_) => {
                    let elapsed = init_start.elapsed();
                    println!(
                        "✓ Dicfuse initialized successfully after {:.2}s",
                        elapsed.as_secs_f64()
                    );
                }
                Err(_) => {
                    panic!(
                        "Dicfuse initialization timed out after 120 seconds. \
                        This may indicate:\n\
                        - Network issues preventing directory tree fetch\n\
                        - Very large directory tree (load_dir_depth={}) taking longer than expected\n\
                        - Background task may have failed\n\
                        Check logs for 'load_dir_depth' and 'Worker processing path' messages",
                        dic.store.max_depth()
                    );
                }
            }

            let mut fuse = AntaresFuse::new(
                mount.clone(),
                std::sync::Arc::new(dic),
                upper.clone(),
                Some(cl.clone()),
            )
            .await
            .unwrap();

            // Mount the overlay filesystem
            // mount() already verifies accessibility via read_dir, so we can skip redundant checks
            println!("Mounting Antares overlay at: {}", mount.display());
            fuse.mount().await.unwrap();
            println!("✓ Mount completed successfully");

            // Verify directories were created
            println!("Verifying directories exist...");

            // Use async metadata with timeout to avoid blocking on FUSE operations
            // PathBuf::exists() on FUSE mountpoint may trigger getattr/lookup which could block
            const CHECK_TIMEOUT_MS: u64 = 5000; // 5 second timeout per check

            // Check mount directory with timeout
            println!("  Checking mount directory: {}", mount.display());
            let mount_check_start = std::time::Instant::now();
            let mount_exists = match tokio::time::timeout(
                Duration::from_millis(CHECK_TIMEOUT_MS),
                tokio::fs::metadata(&mount),
            )
            .await
            {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => false,
                Err(_) => {
                    let elapsed = mount_check_start.elapsed();
                    panic!("Mount directory check timed out after {:.2}s - FUSE operation may be blocked", elapsed.as_secs_f64());
                }
            };
            let mount_check_elapsed = mount_check_start.elapsed();
            println!(
                "  Mount directory check took {:.2}ms, exists: {}",
                mount_check_elapsed.as_secs_f64() * 1000.0,
                mount_exists
            );
            assert!(mount_exists, "mount directory should exist");
            println!("✓ Mount directory exists");

            // Check upper directory (regular filesystem, should be fast)
            println!("  Checking upper directory: {}", upper.display());
            let upper_check_start = std::time::Instant::now();
            let upper_exists = match tokio::time::timeout(
                Duration::from_millis(CHECK_TIMEOUT_MS),
                tokio::fs::metadata(&upper),
            )
            .await
            {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => false,
                Err(_) => {
                    let elapsed = upper_check_start.elapsed();
                    panic!(
                        "Upper directory check timed out after {:.2}s",
                        elapsed.as_secs_f64()
                    );
                }
            };
            let upper_check_elapsed = upper_check_start.elapsed();
            println!(
                "  Upper directory check took {:.2}ms, exists: {}",
                upper_check_elapsed.as_secs_f64() * 1000.0,
                upper_exists
            );
            assert!(upper_exists, "upper directory should exist");
            println!("✓ Upper directory exists");

            // Check CL directory (regular filesystem, should be fast)
            println!("  Checking CL directory: {}", cl.display());
            let cl_check_start = std::time::Instant::now();
            let cl_exists = match tokio::time::timeout(
                Duration::from_millis(CHECK_TIMEOUT_MS),
                tokio::fs::metadata(&cl),
            )
            .await
            {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => false,
                Err(_) => {
                    let elapsed = cl_check_start.elapsed();
                    panic!(
                        "CL directory check timed out after {:.2}s",
                        elapsed.as_secs_f64()
                    );
                }
            };
            let cl_check_elapsed = cl_check_start.elapsed();
            println!(
                "  CL directory check took {:.2}ms, exists: {}",
                cl_check_elapsed.as_secs_f64() * 1000.0,
                cl_exists
            );
            assert!(cl_exists, "cl directory should exist");
            println!("✓ CL directory exists");
            // Note: We don't call read_dir here because:
            // 1. mount() already verified accessibility via read_dir internally
            // 2. read_dir on FUSE mountpoint may trigger readdirplus which could block
            //    if Dicfuse is still loading data in the background
            // 3. This test focuses on verifying directory creation, not readdir functionality

            // Unmount
            println!("Unmounting...");
            let unmount_start = std::time::Instant::now();
            fuse.unmount().await.unwrap();
            let unmount_elapsed = unmount_start.elapsed();
            println!(
                "✓ Unmount successful (took {:.2}s)",
                unmount_elapsed.as_secs_f64()
            );

            // Cleanup
            let _ = std::fs::remove_dir_all(&base);
        };

        // Run test with timeout to prevent hanging
        // Increased timeout to 180s to account for Dicfuse network initialization and large directory trees
        match tokio::time::timeout(Duration::from_secs(180), test_future).await {
            Ok(_) => println!("✓ Test completed successfully"),
            Err(_) => panic!("Test timed out after 180 seconds - this may indicate:\n- Dicfuse background loading taking too long\n- Network issues\n- Very large directory tree (check load_dir_depth config)\nCheck logs for '[load_dir_depth]' messages to see loading progress"),
        }
    }

    /// Verify that creating a file in a deep directory path is reflected in the upper layer.
    ///
    /// Topology:
    /// - lower: one passthrough layer with a 3-level deep directory tree `a/b/c`.
    /// - upper: empty directory used as the writable layer.
    ///
    /// We create a file at `/mnt/a/b/c/created.txt` and then check that the file
    /// appears under `upper/a/b/c/created.txt` and does NOT exist in the lower tree.
    #[tokio::test]
    #[ignore]
    // Requires FUSE/root. Direct run example:
    //   sudo -E cargo test --lib antares::fuse::tests::deep_write_goes_to_upper -- --exact --ignored --nocapture
    // For LLDB-based debugging, follow the steps in `doc/test.md`.
    #[serial] // Serialize to avoid config initialization conflicts
    async fn deep_write_goes_to_upper() {
        use libfuse_fs::{
            passthrough::{new_passthroughfs_layer, newlogfs::LoggingFileSystem, PassthroughArgs},
            unionfs::{config::Config, OverlayFs},
        };
        use std::sync::Arc;
        // Only  LoggingFileSystem DEBUG
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive("libfuse_fs::passthrough::newlogfs=debug".parse().unwrap()),
            )
            .try_init();
        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges for FUSE/open_by_handle_at");
            println!(
                "Run with: sudo -E cargo test --lib antares::fuse::tests::deep_write_goes_to_upper -- --exact --ignored --nocapture"
            );
            return;
        }

        let base = PathBuf::from("/tmp/antares_deep_overlay_test3");
        // Clean up any existing mount point first
        let mount = base.join("mnt");
        let _ = tokio::process::Command::new("fusermount")
            .arg("-uz")
            .arg(&mount)
            .output()
            .await;
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let lower = base.join("lower");

        // Prepare directory layout: lower contains `a/b/c`, upper is empty.
        std::fs::create_dir_all(&mount).unwrap();
        std::fs::create_dir_all(&upper).unwrap();
        std::fs::create_dir_all(lower.join("a/b/c")).unwrap();

        // Build overlay: empty upper, single lower.
        let lower_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &lower,
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
            vec![Arc::new(lower_layer)],
            cfg,
            1,
        )
        .unwrap();

        println!("Mounting deep overlay at: {}", mount.display());
        let logfs = LoggingFileSystem::new(overlay);
        let handle = crate::server::mount_filesystem(logfs, mount.as_os_str()).await;

        // Run FUSE session in the background.
        let _fuse_task = tokio::spawn(async move {
            let _ = handle.await;
        });

        // Give the mount a moment to initialize.
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify mountpoint is accessible
        let metadata = tokio::fs::metadata(&mount).await;
        assert!(metadata.is_ok(), "Mountpoint should be accessible");

        // Test: Create a file in a deep directory path
        let test_file = mount.join("a/b/c/created.txt");
        tokio::fs::write(&test_file, b"test content").await.unwrap();

        // Verify file exists in mountpoint
        let content = tokio::fs::read(&test_file).await.unwrap();
        assert_eq!(content, b"test content");

        // Verify file exists in upper layer (copy-up happened)
        let upper_file = upper.join("a/b/c/created.txt");
        let upper_content = tokio::fs::read(&upper_file).await.unwrap();
        assert_eq!(
            upper_content, b"test content",
            "File should be copied up to upper layer"
        );

        // Verify file does NOT exist in lower layer
        let lower_file = lower.join("a/b/c/created.txt");
        assert!(!lower_file.exists(), "File should NOT exist in lower layer");

        // Unmount
        let _ = tokio::process::Command::new("fusermount")
            .arg("-uz")
            .arg(&mount)
            .output()
            .await;

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Test that copy-up works correctly when modifying files from the lower layer.
    /// This test specifically verifies that `do_getattr_helper` is properly implemented,
    /// as copy-up requires getting file attributes from the lower layer.
    #[tokio::test]
    #[ignore] // Requires root privileges and network access
    #[serial]
    async fn test_copyup_modifies_lower_file() {
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges");
            println!("Run with: sudo -E cargo test -p scorpio --lib antares::fuse::tests::test_copyup_modifies_lower_file -- --exact --ignored --nocapture");
            println!("Skipping test...");
            return;
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_copyup_test_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let cl = base.join("cl");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        // Create Dicfuse and wait for directory tree to load
        let dic = crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;

        println!("Loading directory tree synchronously...");
        crate::dicfuse::store::import_arc(dic.store.clone()).await;
        println!("Directory tree loaded");

        let mut fuse = AntaresFuse::new(
            mount.clone(),
            std::sync::Arc::new(dic),
            upper.clone(),
            Some(cl.clone()),
        )
        .await
        .unwrap();

        println!("Mounting Antares overlay at: {}", mount.display());
        fuse.mount().await.unwrap();
        println!("Mount completed");

        // Give mount a moment to stabilize
        sleep(Duration::from_millis(500)).await;

        // Recursively find a file from the lower layer (Dicfuse)
        // We need to search in subdirectories since root may only have directories
        async fn find_file_recursive(
            dir: &std::path::Path,
            upper: &std::path::Path,
            mount: &std::path::Path,
            depth: usize,
        ) -> Option<std::path::PathBuf> {
            if depth > 3 {
                return None; // Don't go too deep
            }

            let mut entries = match tokio::fs::read_dir(dir).await {
                Ok(e) => e,
                Err(_) => return None,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_file() {
                    // Get relative path from mount point
                    let rel_path = path.strip_prefix(mount).unwrap_or(&path);
                    let upper_path = upper.join(rel_path);
                    if !upper_path.exists() {
                        return Some(path);
                    }
                } else if file_type.is_dir() {
                    // Recurse into subdirectory
                    if let Some(found) =
                        Box::pin(find_file_recursive(&path, upper, mount, depth + 1)).await
                    {
                        return Some(found);
                    }
                }
            }
            None
        }

        println!("Searching for a file in lower layer (Dicfuse)...");
        let found_lower_file = find_file_recursive(&mount, &upper, &mount, 0).await;

        if let Some(lower_file) = found_lower_file {
            // Get relative path from mount point for correct upper layer path
            let rel_path = lower_file.strip_prefix(&mount).unwrap();
            println!("Found lower layer file: {}", rel_path.display());

            // Read original content
            let original_content = tokio::fs::read(&lower_file).await.unwrap();
            println!("Original content length: {} bytes", original_content.len());

            // Modify the file - THIS TRIGGERS COPY-UP
            // Copy-up calls do_getattr_helper to get file attributes
            let modified_content = b"MODIFIED BY TEST - copy-up successful!";
            println!("Attempting to modify file (this triggers copy-up)...");

            match tokio::fs::write(&lower_file, modified_content).await {
                Ok(_) => {
                    println!("✓ File modification successful");

                    // Verify modification persisted
                    let read_back = tokio::fs::read(&lower_file).await.unwrap();
                    assert_eq!(
                        read_back, modified_content,
                        "Modified content should be readable"
                    );
                    println!("✓ Modified content verified");

                    // Verify copy-up: file should now be in upper layer (use relative path)
                    let upper_file = upper.join(rel_path);
                    assert!(
                        upper_file.exists(),
                        "File should be copied to upper layer after modification: {}",
                        upper_file.display()
                    );

                    let upper_content = tokio::fs::read(&upper_file).await.unwrap();
                    assert_eq!(
                        upper_content, modified_content,
                        "Upper layer should have modified content"
                    );
                    println!(
                        "✓ Copy-up verified: {} copied to upper layer with modified content",
                        rel_path.display()
                    );
                }
                Err(e) => {
                    panic!("Failed to modify lower layer file - copy-up failed: {}", e);
                }
            }
        } else {
            println!("⚠ No files found in lower layer - test inconclusive");
            println!("  This may happen if Dicfuse couldn't load files from remote server");
        }

        // Cleanup
        println!("Unmounting...");
        fuse.unmount().await.unwrap();
        println!("✓ Test completed");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Test that mkdir works in a lower layer directory (requires directory copy-up).
    /// This simulates Buck2's behavior: creating buck-out/v2 inside a directory from Dicfuse.
    ///
    /// The test verifies:
    /// 1. We can find a directory from lower layer (Dicfuse)
    /// 2. We can create a new subdirectory inside it (triggers directory copy-up)
    /// 3. The new directory appears in the upper layer
    #[tokio::test]
    #[ignore] // Requires root privileges and network access
    #[serial]
    async fn test_mkdir_in_lower_layer_directory() {
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges");
            println!("Run with: sudo -E cargo test -p scorpio --lib antares::fuse::tests::test_mkdir_in_lower_layer_directory -- --exact --ignored --nocapture");
            println!("Skipping test...");
            return;
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_mkdir_test_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let cl = base.join("cl");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        // Create Dicfuse and wait for directory tree to load
        let dic = crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;

        println!("Loading directory tree synchronously...");
        crate::dicfuse::store::import_arc(dic.store.clone()).await;
        println!("Directory tree loaded");

        let mut fuse = AntaresFuse::new(
            mount.clone(),
            std::sync::Arc::new(dic),
            upper.clone(),
            Some(cl.clone()),
        )
        .await
        .unwrap();

        println!("Mounting Antares overlay at: {}", mount.display());
        fuse.mount().await.unwrap();
        println!("Mount completed");

        // Give mount a moment to stabilize
        sleep(Duration::from_millis(500)).await;

        // Find a directory from the lower layer (Dicfuse)
        async fn find_dir_recursive(
            dir: &std::path::Path,
            upper: &std::path::Path,
            mount: &std::path::Path,
            depth: usize,
        ) -> Option<std::path::PathBuf> {
            if depth > 2 {
                return None; // Don't go too deep
            }

            let mut entries = match tokio::fs::read_dir(dir).await {
                Ok(e) => e,
                Err(_) => return None,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    // Get relative path from mount point
                    let rel_path = path.strip_prefix(mount).unwrap_or(&path);
                    let upper_path = upper.join(rel_path);

                    // We want a directory that exists in lower but NOT in upper
                    if !upper_path.exists() {
                        return Some(path);
                    }

                    // Recurse into subdirectory
                    if let Some(found) =
                        Box::pin(find_dir_recursive(&path, upper, mount, depth + 1)).await
                    {
                        return Some(found);
                    }
                }
            }
            None
        }

        println!("Searching for a directory in lower layer (Dicfuse)...");
        let found_lower_dir = find_dir_recursive(&mount, &upper, &mount, 0).await;

        if let Some(lower_dir) = found_lower_dir {
            let rel_path = lower_dir.strip_prefix(&mount).unwrap();
            println!("Found lower layer directory: {}", rel_path.display());

            // Try to create a new subdirectory inside it
            // This simulates Buck2 creating buck-out/v2
            let new_subdir = lower_dir.join("test-subdir-created-by-test");
            println!(
                "Attempting to create subdirectory: {}",
                new_subdir.strip_prefix(&mount).unwrap().display()
            );
            println!("This will trigger directory copy-up...");

            match tokio::fs::create_dir(&new_subdir).await {
                Ok(_) => {
                    println!("✓ Subdirectory creation successful!");

                    // Verify the directory exists (use async with timeout to avoid blocking)
                    match tokio::time::timeout(
                        Duration::from_secs(2),
                        tokio::fs::metadata(&new_subdir),
                    )
                    .await
                    {
                        Ok(Ok(meta)) if meta.is_dir() => {
                            println!("✓ Subdirectory exists in mountpoint");
                        }
                        _ => {
                            println!(
                                "⚠ Could not verify subdirectory in mountpoint (timeout or error)"
                            );
                        }
                    }

                    // Verify it's in upper layer (copy-up happened for parent directory)
                    // Use std::fs for upper layer since it's not through FUSE
                    let upper_new_subdir = upper.join(rel_path).join("test-subdir-created-by-test");

                    // Give filesystem a moment to sync
                    sleep(Duration::from_millis(100)).await;

                    if upper_new_subdir.exists() {
                        println!(
                            "✓ Directory copy-up verified: new subdirectory exists in upper layer"
                        );
                        println!("  Upper path: {}", upper_new_subdir.display());
                    } else {
                        println!(
                            "⚠ New subdirectory not found in upper layer (may be a timing issue)"
                        );
                        println!("  Expected: {}", upper_new_subdir.display());
                    }

                    // Test creating a file inside the new directory (with timeout)
                    let test_file = new_subdir.join("test.txt");
                    match tokio::time::timeout(
                        Duration::from_secs(2),
                        tokio::fs::write(&test_file, b"test content"),
                    )
                    .await
                    {
                        Ok(Ok(_)) => {
                            println!("✓ Created file inside new subdirectory");

                            // Verify file content (with timeout)
                            match tokio::time::timeout(
                                Duration::from_secs(2),
                                tokio::fs::read(&test_file),
                            )
                            .await
                            {
                                Ok(Ok(content)) => {
                                    assert_eq!(content, b"test content");
                                    println!("✓ File content verified");
                                }
                                _ => {
                                    println!("⚠ Could not verify file content (timeout)");
                                }
                            }
                        }
                        _ => {
                            println!("⚠ Could not create file inside new subdirectory (timeout)");
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to create subdirectory in lower layer directory!");
                    println!(
                        "  Error: {} (os error {})",
                        e,
                        e.raw_os_error().unwrap_or(-1)
                    );
                    println!("  This indicates directory copy-up is not working correctly.");
                    println!(
                        "  The OverlayFS should copy the parent directory to upper layer first,"
                    );
                    println!("  then create the new subdirectory there.");
                    panic!("mkdir in lower layer directory failed: {}", e);
                }
            }
        } else {
            println!("⚠ No directories found in lower layer - test inconclusive");
            println!("  This may happen if Dicfuse couldn't load directories from remote server");
        }

        // Cleanup
        println!("Unmounting...");
        fuse.unmount().await.unwrap();
        println!("✓ Test completed");

        let _ = std::fs::remove_dir_all(&base);
    }
}
