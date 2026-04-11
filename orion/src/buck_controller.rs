use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io::BufReader,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
};

use anyhow::anyhow;
use api_model::buck2::{
    status::Status,
    types::{ProjectRelativePath, TaskPhase},
    ws::{WSBuildContext, WSMessage, WSTargetBuildStatusEvent},
};
use common::config::BuildConfig;
use once_cell::sync::Lazy;
use td_util::{command::spawn, file_io::file_writer, file_tail::tail_compressed_buck2_events};
use td_util_buck::{
    cells::CellInfo,
    run::{Buck2, targets_arguments},
    target_status::{BuildState, EVENT_LOG_FILE, Event, LogicalActionId, TargetBuildStatusUpdate},
    targets::Targets,
    types::TargetLabel,
};
use tokio::{
    io::AsyncBufReadExt,
    process::Command,
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
    time::{Duration, interval},
};
use tokio_util::sync::CancellationToken;

// Import complete Error trait for better error handling
use crate::repo::changes::Changes;
use crate::repo::diff;

const MAX_BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL_MS: u64 = 100;

#[allow(dead_code)]
static PROJECT_ROOT: Lazy<String> =
    Lazy::new(|| std::env::var("BUCK_PROJECT_ROOT").expect("BUCK_PROJECT_ROOT must be set"));

const DEFAULT_PREHEAT_SHALLOW_DEPTH: usize = 3;
static BUILD_CONFIG: Lazy<Option<BuildConfig>> = Lazy::new(load_build_config);

/// Mount an Antares overlay filesystem for a build job.
///
/// Creates a new Antares overlay mount using scorpiofs. The underlying Dicfuse
/// layer provides read-only access to the repository, while the overlay allows
/// copy-on-write modifications during the build.
///
/// # Arguments
/// * `job_id` - Unique identifier for this build job
/// * `cl` - Optional changelist layer name
///
/// # Returns
/// Returns a tuple `(mountpoint, mount_id)` on success.
pub async fn mount_antares_fs(
    job_id: &str,
    cl: Option<&str>,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!(
        "Preparing to mount Antares FS: job_id={}, cl={:?}",
        job_id,
        cl
    );

    let config = crate::antares::mount_job(job_id, cl).await?;

    let mountpoint = config.mountpoint.to_string_lossy().to_string();
    let mount_id = config.job_id.clone();

    tracing::info!(
        "Antares mount created successfully: mountpoint={}, mount_id={}",
        mountpoint,
        mount_id
    );

    Ok((mountpoint, mount_id))
}

/// Unmount an Antares overlay filesystem.
///
/// # Arguments
/// * `mount_id` - The job/mount ID to unmount
///
/// # Returns
/// * `Ok(true)` - Unmount succeeded
/// * `Err` - Unmount failed
#[allow(dead_code)]
pub async fn unmount_antares_fs(mount_id: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
    tracing::info!("Starting unmount for mount_id: {}", mount_id);

    crate::antares::unmount_job(mount_id).await?;

    tracing::info!("Unmount succeeded for mount_id: {}", mount_id);
    Ok(true)
}

/// Pre-warm FUSE metadata by walking the directory tree.
///
/// After the Antares `wait_for_mount_ready` integration, the heavy deep preload
/// is performed by the scorpio daemon itself. This function serves as a secondary
/// fallback to catch any directories that might have been missed or expired.
///
/// We use a shallow Rust-native walk (instead of `ls -lR`) for better control
/// and error resilience.
#[allow(dead_code)]
fn preheat(repo_path: &Path) -> anyhow::Result<()> {
    tracing::info!(repo = ?repo_path, "preheat: starting lightweight metadata warmup");
    let start = std::time::Instant::now();
    let depth = preheat_shallow_depth();
    preheat_shallow(repo_path, depth)?;
    tracing::info!(
        repo = ?repo_path,
        elapsed_ms = start.elapsed().as_millis(),
        depth = depth,
        "preheat: completed"
    );
    Ok(())
}

/// Preheat a shallow directory tree to reduce cold-start metadata misses.
fn preheat_shallow(repo_path: &Path, max_depth: usize) -> anyhow::Result<()> {
    if max_depth == 0 {
        return Ok(());
    }

    let mut stack = vec![(repo_path.to_path_buf(), 0usize)];
    while let Some((path, depth)) = stack.pop() {
        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(err) => {
                tracing::warn!("Preheat shallow read_dir failed for {path:?}: {err}");
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!("Preheat shallow entry error under {path:?}: {err}");
                    continue;
                }
            };

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    tracing::warn!(
                        "Preheat shallow file_type failed for {:?}: {err}",
                        entry.path()
                    );
                    continue;
                }
            };

            // Touch metadata to warm FUSE cache.
            let _ = entry.metadata();

            if file_type.is_dir() && depth < max_depth {
                stack.push((entry.path(), depth + 1));
            }
        }
    }

    Ok(())
}

fn preheat_shallow_depth() -> usize {
    if let Some(depth) = parse_env_usize("ORION_PREHEAT_SHALLOW_DEPTH") {
        return depth;
    }

    if let Some(depth) = parse_env_usize("MEGA_BUILD__ORION_PREHEAT_SHALLOW_DEPTH") {
        return depth;
    }

    build_config()
        .map(|config| config.orion_preheat_shallow_depth)
        .unwrap_or(DEFAULT_PREHEAT_SHALLOW_DEPTH)
}

fn parse_env_usize(key: &str) -> Option<usize> {
    match std::env::var(key) {
        Ok(value) => match value.parse::<usize>() {
            Ok(depth) => Some(depth),
            Err(_) => {
                tracing::warn!("Invalid {key}={value:?}, ignoring.");
                None
            }
        },
        Err(_) => None,
    }
}

fn build_config() -> Option<&'static BuildConfig> {
    BUILD_CONFIG.as_ref()
}

fn load_build_config() -> Option<BuildConfig> {
    let path = resolve_config_path()?;
    let path_str = path.to_str()?;
    match common::config::Config::new(path_str) {
        Ok(config) => Some(config.build),
        Err(err) => {
            tracing::warn!("Failed to load MEGA config from {path:?}: {err}");
            None
        }
    }
}

fn resolve_config_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("MEGA_CONFIG") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
        tracing::warn!("MEGA_CONFIG points to missing file: {path:?}");
    }

    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join("config/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(base_dir) = std::env::var("MEGA_BASE_DIR") {
        let path = PathBuf::from(base_dir).join("etc/config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Get target of a specific repo under tmp directory.
///
/// Note: `preheat()` was previously called here to warm the kernel VFS/FUSE
/// cache, but it has been removed. The new `warmup_dicfuse()` at startup
/// populates the Dicfuse backing store which makes subsequent FUSE reads
/// fast, and `preheat_shallow()` in `get_build_targets()` handles the
/// per-mount VFS cache warming.
fn get_repo_targets(
    file_name: &str,
    repo_path: &Path,
    cells: Option<&CellInfo>,
) -> anyhow::Result<Targets> {
    const MAX_ATTEMPTS: usize = 2;
    let jsonl_path = PathBuf::from(repo_path).join(file_name);

    for attempt in 1..=MAX_ATTEMPTS {
        tracing::debug!("Get targets for repo {repo_path:?} (attempt {attempt}/{MAX_ATTEMPTS})");
        let mut command = std::process::Command::new("buck2");
        command
            .env("BUCKD_STARTUP_TIMEOUT", "30")
            .env("BUCKD_STARTUP_INIT_TIMEOUT", "1200");

        // Add base targets arguments
        command.args(targets_arguments());

        // If cells info is provided, query all cells; otherwise just query root cell
        if let Some(cells_info) = cells {
            let cell_patterns = cells_info.get_all_cell_patterns(repo_path);
            tracing::debug!("Querying targets for cells: {:?}", cell_patterns);
            command.args(&cell_patterns);
        } else {
            // Default: only query root cell
            command.arg("//...");
        }

        command.current_dir(repo_path);
        command.stderr(Stdio::piped());
        let (mut child, stdout) = spawn(command)?;
        let mut stderr = child.stderr.take().expect("stderr should be piped");

        // Consume stderr in background thread to prevent pipe deadlock
        let stderr_handle = std::thread::spawn(move || {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut stderr, &mut buf)
                .map(|_| String::from_utf8_lossy(&buf).to_string())
                .unwrap_or_else(|_| "<failed to read stderr>".to_string())
        });

        let mut writer = file_writer(&jsonl_path)?;
        std::io::copy(&mut BufReader::new(stdout), &mut writer)
            .map_err(|err| anyhow!("Failed to copy output to stdout: {}", err))?;
        writer
            .flush()
            .map_err(|err| anyhow!("Failed to flush writer: {}", err))?;

        let status = child.wait()?;
        let stderr_output = stderr_handle
            .join()
            .unwrap_or_else(|_| "<stderr thread panicked>".to_string());

        if status.success() {
            if !stderr_output.trim().is_empty() {
                tracing::debug!(
                    "buck2 targets succeeded for repo {:?}, stderr: {}",
                    repo_path,
                    stderr_output.trim()
                );
            }
            return Targets::from_file(&jsonl_path);
        }

        tracing::warn!(
            "buck2 targets failed with status {} for repo {:?}. stderr: {}",
            status,
            repo_path,
            stderr_output.trim()
        );
        if attempt < MAX_ATTEMPTS {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    Err(anyhow!(
        "buck2 targets failed after {MAX_ATTEMPTS} attempts for repo {:?}",
        repo_path
    ))
}

fn collect_impacted_targets(base: &Targets, diff: &Targets, changes: &Changes) -> Vec<TargetLabel> {
    let immediate = diff::immediate_target_changes(base, diff, changes, false);
    let recursive = diff::recursive_target_changes(diff, changes, &immediate, None, |_| true);

    let targets: Vec<_> = recursive
        .into_iter()
        .flatten()
        .map(|(target, _)| target.label())
        .collect();

    if targets.is_empty() {
        tracing::info!(
            changes_count = changes.cell_paths().count(),
            base_targets = base.len_targets_upperbound(),
            diff_targets = diff.len_targets_upperbound(),
            "No impacted targets found. Changes may not match any target inputs or packages."
        );
    } else {
        tracing::info!(impacted_targets = targets.len(), "Found impacted targets");
    }

    targets
}

fn has_path_component_suffix(candidate: &str, suffix: &str) -> bool {
    candidate == suffix
        || candidate
            .strip_suffix(suffix)
            .is_some_and(|prefix| prefix.ends_with('/'))
}

fn remap_repo_local_change_paths(
    project_root: &Path,
    diff: &Targets,
    changes: &[Status<ProjectRelativePath>],
) -> (Vec<Status<ProjectRelativePath>>, usize) {
    let mut remapped_count = 0usize;
    let mut normalized_changes = Vec::with_capacity(changes.len());

    for change in changes {
        let original = change.get().as_str();
        if original.is_empty() || project_root.join(original).exists() {
            normalized_changes.push(change.clone());
            continue;
        }

        let mut candidates: HashSet<String> = HashSet::new();
        for target in diff.targets() {
            for input in target.inputs.iter() {
                let candidate_path = input.path();
                let candidate = candidate_path.as_str();
                if !has_path_component_suffix(candidate, original) {
                    continue;
                }
                if project_root.join(candidate).exists() {
                    candidates.insert(candidate.to_owned());
                    if candidates.len() > 1 {
                        break;
                    }
                }
            }
            if candidates.len() > 1 {
                break;
            }
        }

        if candidates.len() == 1 {
            let remapped = candidates.into_iter().next().expect("single candidate");
            tracing::info!(
                original_path = %original,
                remapped_path = %remapped,
                "Remapping unresolved repo-local change path to a unique Buck input path."
            );
            remapped_count += 1;
            normalized_changes.push(
                change
                    .clone()
                    .into_map(|_| ProjectRelativePath::new(&remapped)),
            );
        } else {
            normalized_changes.push(change.clone());
        }
    }

    (normalized_changes, remapped_count)
}

/// Run buck2-change-detector to get targets to build.
///
/// # Note
/// `mount_point` must be a mounted repository or CL path.
/// `mega_changes` follows the hybrid path contract used end-to-end:
/// repo-local files are repo-relative, while shared external paths stay
/// monorepo-relative.
async fn get_build_targets(
    old_repo_mount_point: &str,
    mount_point: &str,
    mega_changes: Vec<Status<ProjectRelativePath>>,
) -> anyhow::Result<Vec<TargetLabel>> {
    tracing::info!("Get cells at {:?}", mount_point);
    let mount_path = PathBuf::from(mount_point);
    let old_repo = PathBuf::from(old_repo_mount_point);
    tracing::debug!("Analyzing changes {mega_changes:?}");

    preheat_shallow(&mount_path, preheat_shallow_depth())?;
    let mut buck2 = Buck2::with_root("buck2".to_string(), mount_path.clone());
    let mut cells = CellInfo::parse(
        &buck2
            .cells()
            .map_err(|err| anyhow!("Fail to get cells: {}", err))?,
    )?;

    tracing::debug!("Get config");
    cells.parse_config_data(
        &buck2
            .audit_config()
            .map_err(|err| anyhow!("Fail to get config: {}", err))?,
    )?;

    let base = get_repo_targets("base.jsonl", &old_repo, Some(&cells))?;
    let diff = get_repo_targets("diff.jsonl", &mount_path, Some(&cells))?;
    let changes = Changes::new(&cells, mega_changes.clone())?;
    tracing::debug!("Changes {changes:?}");

    tracing::debug!("Base targets number: {}", base.len_targets_upperbound());
    tracing::debug!("Diff targets number: {}", diff.len_targets_upperbound());

    let targets = collect_impacted_targets(&base, &diff, &changes);
    if !targets.is_empty() {
        return Ok(targets);
    }

    let (remapped_changes, remapped_count) =
        remap_repo_local_change_paths(&mount_path, &diff, &mega_changes);
    if remapped_count == 0 {
        return Ok(targets);
    }

    let remapped = Changes::new(&cells, remapped_changes)?;
    let remapped_targets = collect_impacted_targets(&base, &diff, &remapped);
    if !remapped_targets.is_empty() {
        tracing::info!(
            remapped_count,
            recovered_target_count = remapped_targets.len(),
            "Recovered impacted Buck targets after remapping repo-local change paths."
        );
    }

    Ok(remapped_targets)
}

fn validate_project_roots(
    old_project_root: &Path,
    new_project_root: &Path,
) -> anyhow::Result<(String, String)> {
    validate_project_root_exists("old", old_project_root)?;
    validate_project_root_exists("new", new_project_root)?;

    Ok((
        path_to_utf8_string("old", old_project_root)?,
        path_to_utf8_string("new", new_project_root)?,
    ))
}

fn validate_project_root_exists(kind: &str, project_root: &Path) -> anyhow::Result<()> {
    if project_root.exists() {
        return Ok(());
    }

    Err(anyhow!(
        "Build repo root ({kind}) does not exist under mounted workspace: {}",
        project_root.display()
    ))
}

fn path_to_utf8_string(kind: &str, path: &Path) -> anyhow::Result<String> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Build repo root ({kind}) contains invalid UTF-8: {path:?}"))
}

fn finish_without_build_if_no_targets(
    build_id: &str,
    targets: &[TargetLabel],
    sender: &UnboundedSender<WSMessage>,
) -> anyhow::Result<bool> {
    if !targets.is_empty() {
        return Ok(false);
    }

    sender
        .send(WSMessage::TaskBuildOutput {
            build_id: build_id.to_string(),
            output: "No impacted Buck targets detected for the provided changes.".to_string(),
        })
        .map_err(|e| anyhow!("Failed to send empty-target build output: {e}"))?;
    Ok(true)
}

fn successful_exit_status() -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        ExitStatus::from_raw(0)
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;

        ExitStatus::from_raw(0)
    }
}

#[derive(Debug)]
pub struct BuildStatusTracker {
    pub cancellation: CancellationToken,
    pub tail_handle: JoinHandle<()>,
    pub process_handle: JoinHandle<()>,
}

/// Start the build status tracker.
/// Returns a `BuildStatusTracker` for lifecycle management.
pub fn start_build_status_tracker(
    project_root: &Path,
    sender: UnboundedSender<WSMessage>,
    cl_id: &str,
    task_id: &str,
) -> BuildStatusTracker {
    let event_jsonl_path = project_root.join(EVENT_LOG_FILE);
    tracing::info!("Track target build status at {:?}", event_jsonl_path);

    let (tx, rx) = mpsc::channel::<String>(4096);
    let cancellation = CancellationToken::new();

    // Spawn tail task: reads the event log file and sends each line into channel
    let tail_handle = {
        let event_log_path = event_jsonl_path.clone();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let poll_interval = Duration::from_millis(200);
            if let Err(e) =
                tail_compressed_buck2_events(event_log_path, tx_clone, poll_interval).await
            {
                tracing::error!("Failed to tail buck2 events: {:?}", e);
            }
        })
    };

    // Spawn processing task: handles events, updates state, flushes WebSocket
    let process_handle = {
        let sender_clone = sender.clone();
        let cancellation_clone = cancellation.clone();
        tokio::spawn(run_processing_loop(
            rx,
            sender_clone,
            cancellation_clone,
            cl_id.to_owned(),
            task_id.to_owned(),
        ))
    };

    BuildStatusTracker {
        cancellation,
        tail_handle,
        process_handle,
    }
}

/// Event processing loop.
/// Reads event lines, updates BuildState, and flushes WebSocket in batches or intervals.
async fn run_processing_loop(
    mut rx: mpsc::Receiver<String>,
    sender: UnboundedSender<WSMessage>,
    cancellation: CancellationToken,
    cl_id: String,
    task_id: String,
) {
    let mut build_state = BuildState::default();
    let mut buffer: HashMap<LogicalActionId, TargetBuildStatusUpdate> = HashMap::with_capacity(256);
    let mut flush_interval = interval(Duration::from_millis(FLUSH_INTERVAL_MS));

    loop {
        tokio::select! {
            Some(line) = rx.recv() => {
                match serde_json::from_str::<Event>(&line) {
                    Ok(event) => {
                        if let Some(update) = build_state.handle_event(&event) {
                            buffer.insert(update.action_id.clone(), update);

                             if buffer.len() >= MAX_BATCH_SIZE
                                && !flush_buffer(&sender, &mut buffer, &cl_id, &task_id).await {
                                    break;
                                }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse event line: {:?}", e);
                    }
                }
            }

            _ = flush_interval.tick() => {
                if !buffer.is_empty()
                   && !flush_buffer(&sender, &mut buffer, &cl_id, &task_id).await {
                        break;
                    }
            }

            _ = cancellation.cancelled() => {
                tracing::info!("Cancellation received, stopping processing loop");
                break;
            }
        }
    }

    // Flush remaining updates
    if !buffer.is_empty() {
        let _ = flush_buffer(&sender, &mut buffer, &cl_id, &task_id).await;
    }

    tracing::info!("Build status processing loop stopped");
}

/// Flush the buffer of updates to the WebSocket.
/// Returns false if the WebSocket is closed.
async fn flush_buffer(
    sender: &UnboundedSender<WSMessage>,
    buffer: &mut HashMap<LogicalActionId, TargetBuildStatusUpdate>,
    cl_id: &str,
    task_id: &str,
) -> bool {
    if buffer.is_empty() {
        tracing::trace!(task_id, cl_id, "Buffer empty, skipping flush");
        return true;
    }

    let update_count = buffer.len();

    tracing::debug!(
        task_id,
        cl_id,
        update_count,
        "Flushing target build status updates"
    );

    let context = WSBuildContext {
        task_id: task_id.to_string(),
        cl_id: cl_id.to_string(),
    };

    let events: Vec<WSTargetBuildStatusEvent> = buffer
        .drain()
        .map(|(_, update)| WSTargetBuildStatusEvent {
            context: context.clone(),
            target: update.into(),
        })
        .collect();

    let message = WSMessage::TargetBuildStatusBatch { events };

    match sender.send(message) {
        Ok(_) => {
            tracing::trace!(
                task_id,
                cl_id,
                update_count,
                "Successfully flushed target build status updates"
            );
            true
        }
        Err(e) => {
            tracing::warn!(
                task_id,
                cl_id,
                update_count,
                error = %e,
                "Failed to send target build status batch"
            );
            false
        }
    }
}

/// Executes buck build with filesystem mounting and output streaming.
///
/// Process flow:
/// 1. Mount repository filesystem via remote API
/// 2. Execute buck build command with specified target and arguments  
/// 3. Stream build output in real-time via WebSocket
/// 4. Return final build status
///
/// # Arguments
/// * `id` - Build task identifier for logging and tracking
/// * `mount_path` - Monorepo path to mount (Buck2 project root or subdirectory)
/// * `target` - Buck build target specification  
/// * `args` - Additional command-line arguments for buck
/// * `cl` - Change List context identifier
/// * `sender` - WebSocket channel for streaming build output
/// * `changes` - Commit's file change information
///
/// # Returns
/// Process exit status indicating build success or failure
pub async fn build(
    id: String,
    repo: String,
    cl: String,
    sender: UnboundedSender<WSMessage>,
    changes: Vec<Status<ProjectRelativePath>>,
) -> Result<ExitStatus, Box<dyn Error + Send + Sync>> {
    tracing::info!("[Task {}] Building in repo {}", id, repo);

    // Until `buck2 build` starts, the worker only had Busy with phase=None. The UI then
    // looks like the client is not reporting progress during FUSE mount + target discovery.
    if let Err(e) = sender.send(WSMessage::TaskPhaseUpdate {
        build_id: id.clone(),
        phase: TaskPhase::DownloadingSource,
    }) {
        tracing::warn!(
            "[Task {}] Failed to send DownloadingSource phase (server may not show prep progress): {}",
            id,
            e
        );
    }

    let task_id = id.trim();
    // Handle empty cl string as None to mount the base repo without a CL layer.
    let cl_trimmed = cl.trim();
    let cl_arg = (!cl_trimmed.is_empty()).then_some(cl_trimmed);

    // `repo_prefix` is the relative path from monorepo root to the buck2 project.
    // e.g., repo="/project/git-internal/git-internal" → repo_prefix="project/git-internal/git-internal"
    let repo_prefix = repo.strip_prefix('/').unwrap_or(&repo);

    // Task changes follow the hybrid path contract:
    // - files inside `repo` are repo-relative
    // - shared files outside `repo` stay monorepo-relative

    const MAX_TARGETS_ATTEMPTS: usize = 2;
    let mut mount_point = None;
    let mut old_repo_mount_point_saved = None;
    let mut targets: Vec<TargetLabel> = Vec::new();
    let mut last_targets_error: Option<anyhow::Error> = None;

    for attempt in 1..=MAX_TARGETS_ATTEMPTS {
        // Mount TWO independent views of the same monorepo:
        //
        //   old_repo  — base revision (no CL), used as the "before" snapshot
        //   new_repo  — base + CL layer,       used as the "after"  snapshot
        //
        // Both mount the same monorepo root (`path = "/"`), but each call to
        // `mount_antares_fs()` creates a **new UUID** on the Antares side, so
        // the mountpoints are different (e.g. `/var/lib/antares/mounts/<uuid>`).
        // Buck2 isolates daemons by project root, so distinct mount paths
        // naturally get separate daemons without needing `--isolation-dir`.
        let id_for_old_repo = format!("{id}-old-{attempt}");
        let (old_repo_mount_point, _mount_id_old_repo) =
            mount_antares_fs(&id_for_old_repo, None).await?;

        let id_for_repo = format!("{id}-{attempt}");
        let (repo_mount_point, _mount_id) = mount_antares_fs(&id_for_repo, cl_arg).await?;

        tracing::info!(
            "[Task {}] Filesystem mounted successfully (attempt {}/{}).",
            id,
            attempt,
            MAX_TARGETS_ATTEMPTS
        );

        // Resolve the sub-project paths within each mount for buck2.
        let old_project_root = PathBuf::from(&old_repo_mount_point).join(repo_prefix);
        let new_project_root = PathBuf::from(&repo_mount_point).join(repo_prefix);

        let project_roots = validate_project_roots(&old_project_root, &new_project_root);
        let (old_project_root_str, new_project_root_str) = match project_roots {
            Ok(roots) => roots,
            Err(e) => {
                tracing::warn!(
                    "[Task {}] Invalid project roots (attempt {}/{}): {}. old_root={}, new_root={}",
                    id,
                    attempt,
                    MAX_TARGETS_ATTEMPTS,
                    e,
                    old_project_root.display(),
                    new_project_root.display(),
                );
                last_targets_error = Some(e);
                if attempt == MAX_TARGETS_ATTEMPTS {
                    break;
                }
                continue;
            }
        };

        match get_build_targets(
            &old_project_root_str,
            &new_project_root_str,
            changes.clone(),
        )
        .await
        {
            Ok(found_targets) => {
                mount_point = Some(repo_mount_point);
                old_repo_mount_point_saved = Some(old_repo_mount_point.clone());
                tracing::info!(
                    "[Task {}] Target discovery succeeded: {} targets",
                    id,
                    found_targets.len()
                );
                targets = found_targets;
                break;
            }
            Err(e) => {
                tracing::warn!(
                    "[Task {}] Failed to get build targets (attempt {}/{}): {}. Mounts retained for debugging (old={}, new={}).",
                    id,
                    attempt,
                    MAX_TARGETS_ATTEMPTS,
                    e,
                    old_repo_mount_point,
                    repo_mount_point,
                );
                last_targets_error = Some(e);
                if attempt == MAX_TARGETS_ATTEMPTS {
                    break;
                }
            }
        }
    }

    let mount_point = match mount_point {
        Some(value) => value,
        None => {
            let err = last_targets_error
                .map(|e| anyhow!("Error getting build targets: {e}"))
                .unwrap_or_else(|| anyhow!("Error getting build targets: mount point missing"));
            let error_msg = err.to_string();
            if sender
                .send(WSMessage::TaskBuildOutput {
                    build_id: id.clone(),
                    output: error_msg.clone(),
                })
                .is_err()
            {
                tracing::error!(
                    "[Task {}] Failed to send BuildOutput for target discovery error",
                    id
                );
            }
            return Err(err.into());
        }
    };

    if finish_without_build_if_no_targets(&id, &targets, &sender)? {
        tracing::info!(
            "[Task {}] No impacted Buck targets detected; skipping buck2 build.",
            id
        );
        return Ok(successful_exit_status());
    }

    let build_result = async {
        // Run buck2 build from the sub-project directory, not the monorepo root.
        // This ensures buck2 uses the sub-project's .buckconfig and PACKAGE files.
        let project_root = PathBuf::from(&mount_point).join(repo_prefix);
        tracing::info!(
            "[Task {}] Starting buck2 build. project_root={}, targets={}",
            id,
            project_root.display(),
            targets.len()
        );

        // Disable both remote and local cache to ensure syntax errors are detected:
        // 1. Kill daemon to clear local action cache
        // 2. Use unique isolation-dir to prevent cache sharing between builds
        // 3. Use --no-remote-cache to prevent remote cache usage
        // Note: Buck2 requires isolation-dir to be a simple directory name without path separators
        let isolation_dir = format!("buck-isolation-{}", id);
        let mut kill_cmd = Command::new("buck2");
        kill_cmd
            .arg("kill")
            .arg("--isolation-dir")
            .arg(&isolation_dir)
            .current_dir(&project_root)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        match kill_cmd.status().await {
            Ok(status) if !status.success() => {
                tracing::debug!("[Task {}] Buck2 daemon was not running (expected)", id);
            }
            Err(e) => {
                tracing::warn!("[Task {}] Failed to kill buck2 daemon: {}", id, e);
            }
            _ => {}
        }

        // Wait for daemon to fully stop before starting a new build
        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut cmd = Command::new("buck2");
        // --event-log and --build-report are used to collect build execution status
        // at target level (e.g. pending / running / succeeded / failed).
        // FUSE-backed repos may trigger lazy loading during daemon init, which
        // can be slow on cold caches — allow up to 1200s for the daemon to start.
        cmd.env("BUCKD_STARTUP_TIMEOUT", "30")
            .env("BUCKD_STARTUP_INIT_TIMEOUT", "1200");
        let cmd = cmd
            .arg("build")
            .args(["--event-log", EVENT_LOG_FILE])
            .args(&targets)
            .arg("--target-platforms")
            .arg("prelude//platforms:default")
            // Avoid failing the whole build when a target is explicitly incompatible
            // with the selected platform (e.g., macOS-only crates on Linux builders).
            .arg("--skip-incompatible-targets")
            .arg("--verbose=2")
            // Disable remote cache to ensure we always build with the latest code changes
            // and detect syntax errors immediately in incremental builds
            .arg("--no-remote-cache")
            .arg("--isolation-dir")
            .arg(&isolation_dir)
            .current_dir(&project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!("[Task {}] Executing command: {:?}", id, cmd);

        let mut child = cmd.spawn().map_err(|e| {
            tracing::error!(
                "[Task {}] Failed to spawn buck2 (cwd={}): {}",
                id,
                project_root.display(),
                e
            );
            e
        })?;

        if let Err(e) = sender.send(WSMessage::TaskPhaseUpdate {
            build_id: id.clone(),
            phase: TaskPhase::RunningBuild,
        }) {
            tracing::error!("Failed to send RunningBuild phase update: {}", e);
        }

        tracing::info!(
            "[Task {}] Starting buck2 event-log tracker: file={}",
            id,
            project_root.join(EVENT_LOG_FILE).display()
        );
        let target_build_track =
            start_build_status_tracker(&project_root, sender.clone(), cl_trimmed, task_id);

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
        let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

        let mut exit_status: Option<ExitStatus> = None;
        loop {
            tokio::select! {
                result = stdout_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            // Log buck2 stdout for debugging
                            tracing::debug!("[Task {}] buck2 stdout: {}", id, line);
                            if sender.send(WSMessage::TaskBuildOutput { build_id: id.clone(), output: line }).is_err() {
                                child.kill().await?;
                                return Err("WebSocket connection lost during build.".into());
                            }
                        },
                        Ok(None) => break,
                        Err(e) => {
                            tracing::error!("[Task {}] Error reading stdout: {}", id, e);
                            break;
                        }
                    }
                },
                result = stderr_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => {
                            // Log buck2 stderr for debugging and error tracking
                            tracing::warn!("[Task {}] buck2 stderr: {}", id, line);
                            if sender.send(WSMessage::TaskBuildOutput { build_id: id.clone(), output: line }).is_err() {
                                child.kill().await?;
                                return Err("WebSocket connection lost during build.".into());
                            }
                        },
                        Ok(None) => break,
                        Err(e) => {
                            tracing::error!("[Task {}] Error reading stderr: {}", id, e);
                            break;
                        },
                    }
                },
                status = child.wait() => {
                    let status = status?;
                    tracing::info!("[Task {}] Buck2 process finished with status: {}", id, status);
                    exit_status = Some(status);
                    break;
                }
            }
        }

        let status = match exit_status {
            Some(s) => s,
            None => child.wait().await?,
        };

        // Log build result for debugging
        if status.success() {
            tracing::info!("[Task {}] Buck2 build completed successfully", id);
        } else {
            tracing::error!(
                "[Task {}] Buck2 build failed with exit code: {}",
                id,
                status.code().map_or("unknown".to_string(), |c| c.to_string())
            );
        }

        // Stop the build-status tracker cleanly.
        // Signal the processing loop to exit via cancellation token; it will
        // flush its remaining buffer in the cleanup path after breaking out of
        // the select! loop.
        target_build_track.cancellation.cancel();
        // Abort the tail task — no more events need to be ingested.
        target_build_track.tail_handle.abort();
        let _ = target_build_track.tail_handle.await;
        // Give the processing loop a short grace period to flush its final
        // buffered updates after observing the cancellation signal.
        if tokio::time::timeout(
            Duration::from_secs(2),
            target_build_track.process_handle,
        )
        .await
        .is_err()
        {
            tracing::warn!("Build status processing loop did not finish within 2s grace period");
        }
        tracing::info!("Target build status track finished");
        Ok(status)
    }
    .await;

    // Cleanup isolation directory to prevent disk space leak
    // Buck2 creates the isolation directory in the project root
    let isolation_dir = format!("buck-isolation-{}", id);
    let isolation_path = PathBuf::from(&mount_point)
        .join(&repo_prefix)
        .join(&isolation_dir);
    if isolation_path.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&isolation_path).await {
            tracing::warn!("[Task {}] Failed to cleanup isolation-dir: {}", id, e);
        }
    }

    tracing::info!(
        "[Task {}] Build completed — mount directories retained for debugging: \
         new_repo mountpoint={}; \
         old_repo mountpoint={}",
        id,
        mount_point,
        old_repo_mount_point_saved.as_deref().unwrap_or("<unknown>"),
    );

    build_result
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use api_model::buck2::{status::Status, types::ProjectRelativePath};
    use serial_test::serial;
    use td_util_buck::types::TargetLabel;
    use tempfile::TempDir;
    use tokio::sync::mpsc;

    use super::{
        finish_without_build_if_no_targets, get_build_targets, get_repo_targets,
        remap_repo_local_change_paths, validate_project_root_exists,
    };

    struct JsonlCleanupGuard {
        paths: Vec<PathBuf>,
    }

    impl JsonlCleanupGuard {
        fn new(paths: impl IntoIterator<Item = PathBuf>) -> Self {
            Self {
                paths: paths.into_iter().collect(),
            }
        }
    }

    impl Drop for JsonlCleanupGuard {
        fn drop(&mut self) {
            for path in &self.paths {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf()
    }

    fn subproject_root(relative: &str) -> PathBuf {
        workspace_root().join(relative)
    }

    fn path_exists(path: &Path) -> bool {
        path.exists()
    }

    fn copy_dir_all(src: &Path, dst: &Path) {
        fs::create_dir_all(dst).expect("create fixture directory");
        for entry in fs::read_dir(src).expect("read fixture directory") {
            let entry = entry.expect("read fixture entry");
            let ty = entry.file_type().expect("read fixture type");
            let dst_path = dst.join(entry.file_name());
            if ty.is_dir() {
                copy_dir_all(&entry.path(), &dst_path);
            } else {
                fs::copy(entry.path(), dst_path).expect("copy fixture file");
            }
        }
    }

    fn isolated_fixture(relative: &str) -> (TempDir, PathBuf, PathBuf) {
        let fixture_root = subproject_root(relative);
        let tempdir = TempDir::new().expect("create tempdir");
        let old_root = tempdir.path().join("old");
        let new_root = tempdir.path().join("new");
        copy_dir_all(&fixture_root, &old_root);
        copy_dir_all(&fixture_root, &new_root);
        (tempdir, old_root, new_root)
    }

    fn isolated_buck_scope_fixture() -> (TempDir, PathBuf, PathBuf) {
        isolated_fixture("orion/tests/fixtures/change_detector_buck_scope")
    }

    fn isolated_ambiguous_main_fixture() -> (TempDir, PathBuf, PathBuf) {
        isolated_fixture("orion/tests/fixtures/change_detector_ambiguous_main")
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_detects_root_relative_subproject_source_change() {
        let subproject_relative = "jupiter/callisto";
        let subproject_root = subproject_root(subproject_relative);
        assert!(
            path_exists(&subproject_root.join("BUCK")),
            "expected test fixture project to exist at {:?}",
            subproject_root
        );

        let _cleanup = JsonlCleanupGuard::new([
            subproject_root.join("base.jsonl"),
            subproject_root.join("diff.jsonl"),
        ]);

        let targets = get_build_targets(
            subproject_root.to_str().expect("subproject root path"),
            subproject_root.to_str().expect("subproject root path"),
            vec![Status::Modified(ProjectRelativePath::new(
                "jupiter/callisto/src/access_token.rs",
            ))],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.contains(&TargetLabel::new("root//jupiter/callisto:callisto")),
            "expected source change to rebuild root//jupiter/callisto:callisto, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_keeps_repo_relative_shared_dependency_changes() {
        let subproject_relative = "orion/tests/fixtures/change_detector_mixed/app";
        let subproject_root = subproject_root(subproject_relative);
        assert!(
            path_exists(&subproject_root.join("BUCK")),
            "expected fixture project to exist at {:?}",
            subproject_root
        );

        let _cleanup = JsonlCleanupGuard::new([
            subproject_root.join("base.jsonl"),
            subproject_root.join("diff.jsonl"),
        ]);

        let targets = get_build_targets(
            subproject_root.to_str().expect("subproject root path"),
            subproject_root.to_str().expect("subproject root path"),
            vec![Status::Modified(ProjectRelativePath::new(
                "orion/tests/fixtures/change_detector_mixed/shared/src/lib.rs",
            ))],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.contains(&TargetLabel::new(
                "root//orion/tests/fixtures/change_detector_mixed/app:app"
            )),
            "expected shared dependency change to rebuild root//orion/tests/fixtures/change_detector_mixed/app:app, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_handles_mixed_repo_root_relative_changes() {
        let subproject_relative = "orion/tests/fixtures/change_detector_mixed/app";
        let subproject_root = subproject_root(subproject_relative);
        assert!(
            path_exists(&subproject_root.join("README.md")),
            "expected fixture file to exist at {:?}",
            subproject_root.join("README.md")
        );

        let _cleanup = JsonlCleanupGuard::new([
            subproject_root.join("base.jsonl"),
            subproject_root.join("diff.jsonl"),
        ]);

        let targets = get_build_targets(
            subproject_root.to_str().expect("subproject root path"),
            subproject_root.to_str().expect("subproject root path"),
            vec![
                Status::Modified(ProjectRelativePath::new(
                    "orion/tests/fixtures/change_detector_mixed/app/README.md",
                )),
                Status::Modified(ProjectRelativePath::new(
                    "orion/tests/fixtures/change_detector_mixed/shared/src/lib.rs",
                )),
            ],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.contains(&TargetLabel::new(
                "root//orion/tests/fixtures/change_detector_mixed/app:app"
            )),
            "expected mixed change list to rebuild root//orion/tests/fixtures/change_detector_mixed/app:app, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_detects_modified_tracked_file_in_standalone_repo() {
        let (_tempdir, old_root, new_root) = isolated_buck_scope_fixture();
        fs::write(
            new_root.join("src/main.rs"),
            "fn main() { println!(\"modified fixture main\"); }\n",
        )
        .expect("rewrite main.rs");

        let targets = get_build_targets(
            old_root.to_str().expect("old fixture path"),
            new_root.to_str().expect("new fixture path"),
            vec![Status::Modified(ProjectRelativePath::new("src/main.rs"))],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.contains(&TargetLabel::new("root//:explicit_main")),
            "expected tracked main.rs change to rebuild root//:explicit_main, got {targets:?}"
        );
        assert!(
            targets.contains(&TargetLabel::new("root//:globbed_lib")),
            "expected tracked main.rs change to rebuild root//:globbed_lib, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_remaps_truncated_repo_local_path() {
        let (_tempdir, old_root, new_root) = isolated_buck_scope_fixture();
        fs::write(
            new_root.join("src/main.rs"),
            "fn main() { println!(\"remapped fixture main\"); }\n",
        )
        .expect("rewrite main.rs");

        let targets = get_build_targets(
            old_root.to_str().expect("old fixture path"),
            new_root.to_str().expect("new fixture path"),
            vec![Status::Modified(ProjectRelativePath::new("main.rs"))],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.contains(&TargetLabel::new("root//:explicit_main")),
            "expected remapped main.rs change to rebuild root//:explicit_main, got {targets:?}"
        );
        assert!(
            targets.contains(&TargetLabel::new("root//:globbed_lib")),
            "expected remapped main.rs change to rebuild root//:globbed_lib, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_detects_added_file_inside_buck_glob_scope() {
        let (_tempdir, old_root, new_root) = isolated_buck_scope_fixture();
        let new_file = new_root.join("src/generated/new_module.rs");
        fs::create_dir_all(
            new_file
                .parent()
                .expect("new file should have a parent directory"),
        )
        .expect("create generated dir");
        fs::write(new_file, "pub fn generated() -> &'static str { \"ok\" }\n")
            .expect("write generated rust file");

        let targets = get_build_targets(
            old_root.to_str().expect("old fixture path"),
            new_root.to_str().expect("new fixture path"),
            vec![Status::Added(ProjectRelativePath::new(
                "src/generated/new_module.rs",
            ))],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.contains(&TargetLabel::new("root//:globbed_lib")),
            "expected added globbed source to rebuild root//:globbed_lib, got {targets:?}"
        );
        assert!(
            !targets.contains(&TargetLabel::new("root//:explicit_main")),
            "explicit target should not rebuild for unrelated added file, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_ignores_added_file_outside_buck_scope() {
        let (_tempdir, old_root, new_root) = isolated_buck_scope_fixture();
        let new_file = new_root.join("notes/added.txt");
        fs::create_dir_all(
            new_file
                .parent()
                .expect("new file should have a parent directory"),
        )
        .expect("create notes dir");
        fs::write(new_file, "not referenced by any Buck target\n").expect("write non-buck file");

        let targets = get_build_targets(
            old_root.to_str().expect("old fixture path"),
            new_root.to_str().expect("new fixture path"),
            vec![Status::Added(ProjectRelativePath::new("notes/added.txt"))],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.is_empty(),
            "expected no targets for out-of-scope added file, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_filters_unsafe_paths_and_keeps_valid_change() {
        let (_tempdir, old_root, new_root) = isolated_buck_scope_fixture();
        fs::write(
            new_root.join("src/main.rs"),
            "fn main() { println!(\"unsafe-filter fixture main\"); }\n",
        )
        .expect("rewrite main.rs");

        let targets = get_build_targets(
            old_root.to_str().expect("old fixture path"),
            new_root.to_str().expect("new fixture path"),
            vec![
                Status::Modified(ProjectRelativePath::new("../secret.rs")),
                Status::Modified(ProjectRelativePath::new("project//buck2_test/src/main.rs")),
                Status::Modified(ProjectRelativePath::new("src/main.rs")),
            ],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.contains(&TargetLabel::new("root//:explicit_main")),
            "expected valid repo-local change to rebuild root//:explicit_main, got {targets:?}"
        );
        assert!(
            targets.contains(&TargetLabel::new("root//:globbed_lib")),
            "expected valid repo-local change to rebuild root//:globbed_lib, got {targets:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_remap_repo_local_change_paths_preserves_removed_and_added_statuses() {
        let (_tempdir, _old_root, new_root) = isolated_buck_scope_fixture();
        let jsonl_cleanup =
            JsonlCleanupGuard::new([new_root.join("diff.jsonl"), new_root.join("base.jsonl")]);
        let diff = get_repo_targets("diff.jsonl", &new_root, None).expect("load diff targets");

        let (remapped, remapped_count) = remap_repo_local_change_paths(
            &new_root,
            &diff,
            &[
                Status::Removed(ProjectRelativePath::new("main.rs")),
                Status::Added(ProjectRelativePath::new("lib.rs")),
            ],
        );

        drop(jsonl_cleanup);
        assert_eq!(remapped_count, 2);
        assert_eq!(
            remapped,
            vec![
                Status::Removed(ProjectRelativePath::new("src/main.rs")),
                Status::Added(ProjectRelativePath::new("src/lib.rs")),
            ]
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_does_not_remap_ambiguous_truncated_path() {
        let (_tempdir, old_root, new_root) = isolated_ambiguous_main_fixture();
        fs::write(
            new_root.join("src/main.rs"),
            "fn main() { println!(\"ambiguous src main\"); }\n",
        )
        .expect("rewrite src/main.rs");
        fs::write(
            new_root.join("examples/main.rs"),
            "pub fn run() { println!(\"ambiguous examples main\"); }\n",
        )
        .expect("rewrite examples/main.rs");

        let targets = get_build_targets(
            old_root.to_str().expect("old fixture path"),
            new_root.to_str().expect("new fixture path"),
            vec![Status::Modified(ProjectRelativePath::new("main.rs"))],
        )
        .await
        .expect("target discovery should complete");

        assert!(
            targets.is_empty(),
            "expected ambiguous short path to avoid remap and keep target list empty, got {targets:?}"
        );
    }

    #[test]
    fn test_finish_without_build_if_no_targets_emits_clear_message() {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let skipped = finish_without_build_if_no_targets("build-1", &[], &sender)
            .expect("empty targets should short-circuit");

        assert!(skipped);
        match receiver.try_recv().expect("expected a websocket message") {
            api_model::buck2::ws::WSMessage::TaskBuildOutput { build_id, output } => {
                assert_eq!(build_id, "build-1");
                assert_eq!(
                    output,
                    "No impacted Buck targets detected for the provided changes."
                );
            }
            other => panic!("unexpected websocket message: {other:?}"),
        }
    }

    #[test]
    fn test_validate_project_root_exists_returns_error_for_missing_path() {
        let tempdir = TempDir::new().expect("create tempdir");
        let missing = tempdir.path().join("missing/subproject");

        let err = validate_project_root_exists("new", &missing).unwrap_err();
        assert!(
            err.to_string()
                .contains("Build repo root (new) does not exist"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_build_command_includes_no_remote_cache_flag() {
        // Read the source code file to verify the flag exists
        let source = include_str!("buck_controller.rs");

        // Verify build function includes --no-remote-cache
        assert!(
            source.contains(r#".arg("--no-remote-cache")"#),
            "buck2 build command must include --no-remote-cache flag. \
             This flag ensures incremental builds always use the latest code changes \
             and detect syntax errors immediately."
        );

        // Verify the comment exists to ensure future maintainers understand why
        assert!(
            source.contains(
                "Disable remote cache to ensure we always build with the latest code changes"
            ),
            "The --no-remote-cache flag must have a comment explaining why it's needed"
        );
    }
}
