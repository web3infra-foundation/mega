use std::path::PathBuf;

use anyhow::Result;
use tokio::fs;
use tracing::info;

use crate::{
    handlers::ImageParams,
    keep_alive::{ImageSpec, KeepAliveMachine},
    state::{AppState, VmInfo},
    vm_manager,
};

/// Handle the update request from GitHub Actions (keep_alive mode)
/// VM stays running after this call.
///
/// If `image_params` is `Some`, the image configuration from the webhook payload
/// takes precedence over the `default_image` in config.
/// If `image_params` is `None`, uses the config-based `default_image`.
pub async fn handle_update(
    state: &AppState,
    target: &str,
    image_params: Option<ImageParams>,
) -> Result<String> {
    info!("Handling update request, target: {}", target);

    // Serialize the entire shutdown/create/set sequence. Holding this guard
    // across the long-running VM build is intentional: any concurrent
    // /webhook request must wait so we never have two `handle_update`
    // bodies racing past the existing-VM check, leaking an orphan qemu
    // that /shutdown cannot reach.
    let _update_guard = state.lock_update().await;
    info!("[orion-deploy] Acquired update lock for target: {}", target);

    // Step 1: Get configuration from config store
    let config = state.config.read().await;
    let target_config = match config.get(target) {
        Some(cfg) => cfg.clone(),
        None => {
            let available = config.target_names();
            return Err(anyhow::anyhow!(
                "Target '{}' not found in config. Available targets: {:?}",
                target,
                available
            ));
        }
    };
    let log_dir = config.log_dir().to_string();
    let orion_source_dir = config.orion_source_dir().to_string();
    let orion_binary_path = config.orion_binary_path().to_string();
    let ssh_public_key_path = config.ssh_public_key_path().to_string();
    drop(config);

    // Step 2: Check if we have an existing VM and shut it down
    if let Some(existing_vm) = state.get_vm().await {
        info!("Found existing VM {}, shutting down", existing_vm.id);
        if let Some(machine) = state.get_machine().await {
            machine.shutdown().await.ok();
        }
        state.clear_vm().await;
    }

    // Step 3: Create new VM (keep_alive mode)
    let vm_name = format!("orion-vm-{}", chrono_lite_timestamp());
    info!("Creating new VM in keep_alive mode: {}", vm_name);

    // Step 3: Build ImageConfig from webhook params (API is the only source of truth)
    let (image_config, disk_gb, cpus, memory_mb) = match &image_params {
        Some(params) => {
            let path = params.path.as_ref();
            let url = params.url.as_ref();
            let digest = params.digest.as_ref();

            // Validate: path and url are mutually exclusive
            if path.is_some() && url.is_some() {
                return Err(anyhow::anyhow!(
                    "image_path and image_url cannot both be set"
                ));
            }

            // Validate: if either path or url is set, digest is required
            if (path.is_some() || url.is_some()) && digest.is_none() {
                return Err(anyhow::anyhow!(
                    "image_digest is required when image_path or image_url is provided"
                ));
            }

            let img_spec = match (url, path, digest) {
                (Some(url), None, Some(digest)) => {
                    info!("[orion-deploy] Using image from URL: {}", url);
                    Some(ImageSpec {
                        source: Some(url.clone()),
                        digest: Some(digest.clone()),
                    })
                }
                (None, Some(path), Some(digest)) => {
                    info!("[orion-deploy] Using image from path: {}", path);
                    Some(ImageSpec {
                        source: Some(path.clone()),
                        digest: Some(digest.clone()),
                    })
                }
                (None, None, _) => {
                    info!("[orion-deploy] No image source in params, using default Debian image");
                    None
                }
                _ => unreachable!(),
            };

            (img_spec, params.disk_gb, params.cpus, params.memory_mb)
        }
        None => {
            info!("[orion-deploy] No image params provided, using default Debian image");
            (None, None, None, None)
        }
    };

    let machine = KeepAliveMachine::new(&vm_name, image_config, disk_gb, cpus, memory_mb).await?;

    // Step 4: Inject SSH keys for debugging
    vm_manager::inject_ssh_keys(&machine, &ssh_public_key_path).await?;

    // Step 5: Deploy Orion files (Buck2 is pre-installed in custom image)
    info!("[orion-deploy] Starting Orion deployment");
    vm_manager::deploy_orion_in_vm(&machine, &orion_source_dir, &orion_binary_path).await?;

    // Step 7: Replace environment variables based on target config
    vm_manager::replace_env_vars_in_vm(&machine, &target_config, target).await?;

    // Step 8: Start Orion and capture initial logs
    let logs = vm_manager::start_orion_in_vm(&machine).await?;

    // Save logs to file
    let log_file = save_orion_logs(&log_dir, &vm_name, &logs).await?;

    // Step 9: Get VM IP address
    let vm_ip = machine.get_ip().await.ok().flatten();
    info!("[orion-deploy] VM IP: {:?}", vm_ip);

    // Set state with VM info and keep-alive machine
    let vm_info = VmInfo {
        id: vm_name.clone(),
        ip: vm_ip,
        created_at: std::time::Instant::now(),
        log_file: Some(log_file.clone()),
    };
    state.set_vm(vm_info, machine).await;

    info!("Update completed successfully for target: {}", target);
    Ok(vm_name)
}

/// Get live Orion logs from the running VM (journalctl + orion.log)
pub async fn get_live_logs(state: &AppState) -> Result<String> {
    let machine = state
        .get_machine()
        .await
        .ok_or_else(|| anyhow::anyhow!("No VM is currently running"))?;

    info!("[orion-deploy] Fetching live Orion logs");

    // Get recent journalctl logs
    let output = machine
        .exec("journalctl -u orion-runner --no-pager -n 100 2>&1")
        .await?;

    // Get Orion log file content
    let orion_log_output = machine.exec("tail -100 /home/orion/orion-runner/log/orion.log 2>/dev/null || echo 'Orion log not found'").await?;

    // Get process info
    let process_output = machine
        .exec("pgrep -a orion || echo 'Orion process not found'")
        .await?;

    let journal_logs = String::from_utf8_lossy(&output.stdout);
    let orion_logs = String::from_utf8_lossy(&orion_log_output.stdout);
    let process_info = String::from_utf8_lossy(&process_output.stdout);

    let logs = format!(
        "{}\n\n========== Orion Log (/home/orion/orion-runner/log/orion.log) ==========\n{}\n\n[Orion Process Info]\n{}",
        journal_logs, orion_logs, process_info
    );

    Ok(logs)
}

/// Get current VM status
pub async fn get_status(state: &AppState) -> Option<VmInfo> {
    state.get_vm().await
}

/// Get Scorpio mount status and directory information
pub async fn get_scorpio_status(state: &AppState) -> Result<serde_json::Value> {
    let machine = state
        .get_machine()
        .await
        .ok_or_else(|| anyhow::anyhow!("No VM is currently running"))?;

    info!("[scorpio] Checking mount status and directories");

    // Define the paths to check
    let paths = vec![
        ("workspace", "/workspace/mount"),
        ("store_path", "/data/scorpio/store"),
        ("antares_upper", "/data/scorpio/antares/upper"),
        ("antares_cl", "/data/scorpio/antares/cl"),
        ("antares_mount", "/data/scorpio/antares/mnt"),
    ];

    let mut results = serde_json::Map::new();

    for (name, path) in paths {
        // Check if path exists
        let exists_output = machine
            .exec(&format!(
                "test -e {} && echo 'exists' || echo 'not_found'",
                path
            ))
            .await?;
        let exists = String::from_utf8_lossy(&exists_output.stdout).contains("exists");

        // Check if it's a mount point
        let mount_output = machine
            .exec(&format!(
                "mountpoint -q {} && echo 'mounted' || echo 'not_mounted'",
                path
            ))
            .await?;
        let is_mounted = String::from_utf8_lossy(&mount_output.stdout).contains("mounted");

        // Get file count if directory exists
        let file_count = if exists {
            let count_output = machine
                .exec(&format!(
                    "find {} -maxdepth 1 -type f 2>/dev/null | wc -l",
                    path
                ))
                .await?;
            String::from_utf8_lossy(&count_output.stdout)
                .trim()
                .to_string()
        } else {
            "N/A".to_string()
        };

        // Get directory count
        let dir_count = if exists {
            let count_output = machine
                .exec(&format!(
                    "find {} -maxdepth 1 -type d 2>/dev/null | wc -l",
                    path
                ))
                .await?;
            String::from_utf8_lossy(&count_output.stdout)
                .trim()
                .to_string()
        } else {
            "N/A".to_string()
        };

        // List contents if directory exists
        let contents: String = if exists {
            let ls_output = machine
                .exec(&format!("ls -la {} 2>/dev/null | head -20", path))
                .await?;
            String::from_utf8_lossy(&ls_output.stdout).into_owned()
        } else {
            "N/A".to_string()
        };

        results.insert(
            name.to_string(),
            serde_json::json!({
                "path": path,
                "exists": exists,
                "is_mounted": is_mounted,
                "file_count": file_count,
                "dir_count": dir_count,
                "contents": contents
            }),
        );
    }

    // Get overall mount status
    let mount_output = machine.exec("mount | grep -E '(workspace|megadir|scorpio|antares)' || echo 'No relevant mounts found'").await?;
    let mount_info = String::from_utf8_lossy(&mount_output.stdout);

    // Get Orion process status
    let orion_output = machine
        .exec("pgrep -a orion || echo 'Orion not running'")
        .await?;
    let orion_info = String::from_utf8_lossy(&orion_output.stdout);

    // Get Scorpio process (if running)
    let scorpio_output = machine
        .exec("pgrep -a scorpio || echo 'Scorpio not running'")
        .await?;
    let scorpio_info = String::from_utf8_lossy(&scorpio_output.stdout);

    // Test network connectivity to git.gitmega.com
    let network_test = machine.exec("curl -sI --connect-timeout 5 https://git.gitmega.com 2>&1 | head -5 || echo 'Connection failed'").await?;
    let network_info = String::from_utf8_lossy(&network_test.stdout);

    let status = serde_json::json!({
        "status": "ok",
        "directories": results,
        "mounts": mount_info,
        "orion_process": orion_info,
        "scorpio_process": scorpio_info,
        "network_test": {
            "git.gitmega.com": network_info.trim()
        }
    });

    Ok(status)
}

/// Save Orion logs to a file in the log directory
async fn save_orion_logs(log_dir: &str, vm_name: &str, logs: &str) -> Result<String> {
    fs::create_dir_all(log_dir).await?;

    let log_file_name = format!("{}-{}.log", vm_name, chrono_lite_timestamp());
    let log_file_path = PathBuf::from(log_dir).join(&log_file_name);

    fs::write(&log_file_path, logs).await?;

    info!("[orion-logs] Saved logs to: {}", log_file_path.display());
    Ok(log_file_path.to_string_lossy().into_owned())
}

/// Generate a Unix timestamp string (seconds since epoch)
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}
