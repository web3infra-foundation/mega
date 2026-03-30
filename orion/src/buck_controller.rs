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
    types::{CellPath, TargetLabel},
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
fn get_repo_targets(file_name: &str, repo_path: &Path) -> anyhow::Result<Targets> {
    const MAX_ATTEMPTS: usize = 2;
    let jsonl_path = PathBuf::from(repo_path).join(file_name);

    for attempt in 1..=MAX_ATTEMPTS {
        tracing::debug!("Get targets for repo {repo_path:?} (attempt {attempt}/{MAX_ATTEMPTS})");
        let mut command = std::process::Command::new("buck2");
        command
            .env("BUCKD_STARTUP_TIMEOUT", "30")
            .env("BUCKD_STARTUP_INIT_TIMEOUT", "1200")
            .args(targets_arguments());
        command.current_dir(repo_path);
        let (mut child, stdout) = spawn(command)?;
        let mut writer = file_writer(&jsonl_path)?;
        std::io::copy(&mut BufReader::new(stdout), &mut writer)
            .map_err(|err| anyhow!("Failed to copy output to stdout: {}", err))?;
        writer
            .flush()
            .map_err(|err| anyhow!("Failed to flush writer: {}", err))?;
        let status = child.wait()?;
        if status.success() {
            return Targets::from_file(&jsonl_path);
        }

        tracing::warn!(
            "buck2 targets failed with status {} for repo {:?}",
            status,
            repo_path
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

/// Run buck2-change-detector to get targets to build.
///
/// # Note
/// `mount_point` must be a mounted repository or CL path.
async fn get_build_targets(
    old_repo_mount_point: &str,
    mount_point: &str,
    repo_prefix: &str,
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

    let base = get_repo_targets("base.jsonl", &old_repo)?;
    let diff = get_repo_targets("diff.jsonl", &mount_path)?;
    let known_paths = collect_known_change_paths(&base, &diff);
    let old_repo_root = repo_root_for_project_root(&old_repo, repo_prefix);
    let new_repo_root = repo_root_for_project_root(&mount_path, repo_prefix);
    let normalized_changes = normalize_changes_for_repo_prefix(
        &cells,
        repo_prefix,
        &old_repo_root,
        &new_repo_root,
        &known_paths,
        mega_changes,
    );
    let changes = Changes::new(&cells, normalized_changes)?;
    tracing::debug!("Changes {changes:?}");

    tracing::debug!("Base targets number: {}", base.len_targets_upperbound());
    tracing::debug!("Diff targets number: {}", diff.len_targets_upperbound());

    let immediate = diff::immediate_target_changes(&base, &diff, &changes, false);
    let recursive = diff::recursive_target_changes(&diff, &changes, &immediate, None, |_| true);

    Ok(recursive
        .into_iter()
        .flatten()
        .map(|(target, _)| target.label())
        .collect())
}

fn collect_known_change_paths(base: &Targets, diff: &Targets) -> HashSet<CellPath> {
    let mut known_paths = HashSet::new();

    for targets in [base, diff] {
        for target in targets.targets() {
            known_paths.extend(target.inputs.iter().cloned());
        }
        for import in targets.imports() {
            known_paths.insert(import.file.clone());
            known_paths.extend(import.imports.iter().cloned());
        }
    }

    known_paths
}

fn repo_root_for_project_root(project_root: &Path, repo_prefix: &str) -> PathBuf {
    let mut repo_root = project_root.to_path_buf();
    for _ in repo_prefix
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
    {
        repo_root = repo_root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(repo_root);
    }
    repo_root
}

fn normalize_changes_for_repo_prefix(
    cells: &CellInfo,
    repo_prefix: &str,
    old_repo_root: &Path,
    new_repo_root: &Path,
    known_paths: &HashSet<CellPath>,
    mega_changes: Vec<Status<ProjectRelativePath>>,
) -> Vec<Status<ProjectRelativePath>> {
    let normalized_prefix = repo_prefix.trim_matches('/');
    let mut normalized_changes = Vec::new();
    let mut seen = HashSet::new();

    for status in mega_changes {
        let candidates = normalize_change_path_candidates(
            cells,
            normalized_prefix,
            old_repo_root,
            new_repo_root,
            known_paths,
            status.get(),
        );
        for candidate in candidates {
            let normalized_status = status_with_path(&status, candidate);
            if seen.insert(normalized_status.clone()) {
                normalized_changes.push(normalized_status);
            }
        }
    }

    normalized_changes
}

fn normalize_change_path_candidates(
    cells: &CellInfo,
    repo_prefix: &str,
    old_repo_root: &Path,
    new_repo_root: &Path,
    known_paths: &HashSet<CellPath>,
    path: &ProjectRelativePath,
) -> Vec<ProjectRelativePath> {
    let raw_path = path.as_str().trim_start_matches('/');
    if repo_prefix.is_empty()
        || raw_path == repo_prefix
        || raw_path.starts_with(&format!("{repo_prefix}/"))
    {
        return vec![ProjectRelativePath::new(raw_path)];
    }

    let prefixed_path = format!("{repo_prefix}/{raw_path}");
    let raw_matches = path_matches_repo(cells, known_paths, old_repo_root, new_repo_root, raw_path);
    let prefixed_matches = path_matches_repo(
        cells,
        known_paths,
        old_repo_root,
        new_repo_root,
        &prefixed_path,
    );

    if raw_matches && prefixed_matches {
        tracing::warn!(
            raw_path,
            prefixed_path,
            repo_prefix,
            "Change path matches both repo-relative and subproject-relative candidates; keeping both"
        );
    }

    select_change_path_candidates(raw_path, &prefixed_path, raw_matches, prefixed_matches)
}

fn path_matches_repo(
    cells: &CellInfo,
    known_paths: &HashSet<CellPath>,
    old_repo_root: &Path,
    new_repo_root: &Path,
    relative_path: &str,
) -> bool {
    path_exists_in_repo(old_repo_root, relative_path)
        || path_exists_in_repo(new_repo_root, relative_path)
        || path_matches_known_targets(cells, known_paths, relative_path)
}

fn path_exists_in_repo(repo_root: &Path, relative_path: &str) -> bool {
    repo_root.join(relative_path).exists()
}

fn path_matches_known_targets(
    cells: &CellInfo,
    known_paths: &HashSet<CellPath>,
    relative_path: &str,
) -> bool {
    cells
        .unresolve(&ProjectRelativePath::new(relative_path))
        .ok()
        .is_some_and(|cell_path| known_paths.contains(&cell_path))
}

fn select_change_path_candidates(
    raw_path: &str,
    prefixed_path: &str,
    raw_matches: bool,
    prefixed_matches: bool,
) -> Vec<ProjectRelativePath> {
    match (raw_matches, prefixed_matches) {
        (false, true) => vec![ProjectRelativePath::new(prefixed_path)],
        (true, false) => vec![ProjectRelativePath::new(raw_path)],
        (true, true) => vec![
            ProjectRelativePath::new(raw_path),
            ProjectRelativePath::new(prefixed_path),
        ],
        (false, false) => vec![ProjectRelativePath::new(raw_path)],
    }
}

fn status_with_path(
    status: &Status<ProjectRelativePath>,
    path: ProjectRelativePath,
) -> Status<ProjectRelativePath> {
    match status {
        Status::Modified(_) => Status::Modified(path),
        Status::Added(_) => Status::Added(path),
        Status::Removed(_) => Status::Removed(path),
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

    let task_id = id.trim();
    // Handle empty cl string as None to mount the base repo without a CL layer.
    let cl_trimmed = cl.trim();
    let cl_arg = (!cl_trimmed.is_empty()).then_some(cl_trimmed);

    // `repo_prefix` is the relative path from monorepo root to the buck2 project.
    // e.g., repo="/project/git-internal/git-internal" → repo_prefix="project/git-internal/git-internal"
    let repo_prefix = repo.strip_prefix('/').unwrap_or(&repo);

    // Buck2 still resolves cells and target inputs relative to the monorepo root
    // even when commands run from a sub-project directory, so normalize incoming
    // change paths against `repo_prefix` before target discovery.

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

        match get_build_targets(
            old_project_root.to_str().unwrap_or(&old_repo_mount_point),
            new_project_root.to_str().unwrap_or(&repo_mount_point),
            repo_prefix,
            changes.clone(),
        )
        .await
        {
            Ok(found_targets) => {
                mount_point = Some(repo_mount_point);
                old_repo_mount_point_saved = Some(old_repo_mount_point.clone());
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

    let build_result = async {
        // Run buck2 build from the sub-project directory, not the monorepo root.
        // This ensures buck2 uses the sub-project's .buckconfig and PACKAGE files.
        let project_root = PathBuf::from(&mount_point).join(repo_prefix);
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
            .current_dir(&project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::debug!("[Task {}] Executing command: {:?}", id, cmd);

        let mut child = cmd.spawn()?;

        if let Err(e) = sender.send(WSMessage::TaskPhaseUpdate {
            build_id: id.clone(),
            phase: TaskPhase::RunningBuild,
        }) {
            tracing::error!("Failed to send RunningBuild phase update: {}", e);
        }

        let target_build_track = start_build_status_tracker(&project_root, sender.clone(), cl_trimmed, task_id);

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
        collections::HashSet,
        fs,
        path::{Path, PathBuf},
    };

    use api_model::buck2::{status::Status, types::ProjectRelativePath};
    use serial_test::serial;
    use td_util_buck::{cells::CellInfo, types::TargetLabel};

    use super::{
        get_build_targets, normalize_change_path_candidates, select_change_path_candidates,
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

    #[test]
    fn test_select_change_path_candidates_prefixes_subproject_relative_paths() {
        let normalized = select_change_path_candidates(
            "src/access_token.rs",
            "jupiter/callisto/src/access_token.rs",
            false,
            true,
        );

        assert_eq!(
            normalized,
            vec![ProjectRelativePath::new(
                "jupiter/callisto/src/access_token.rs"
            )]
        );
    }

    #[test]
    fn test_normalize_change_path_candidates_keeps_repo_relative_paths_idempotent() {
        let normalized = normalize_change_path_candidates(
            &CellInfo::testing(),
            "jupiter/callisto",
            &workspace_root(),
            &workspace_root(),
            &HashSet::new(),
            &ProjectRelativePath::new("jupiter/callisto/src/access_token.rs"),
        );

        assert_eq!(
            normalized,
            vec![ProjectRelativePath::new(
                "jupiter/callisto/src/access_token.rs"
            )]
        );
    }

    #[test]
    fn test_select_change_path_candidates_keeps_unrelated_repo_relative_paths_unchanged() {
        let normalized = select_change_path_candidates(
            "common/src/lib.rs",
            "jupiter/callisto/common/src/lib.rs",
            true,
            false,
        );

        assert_eq!(
            normalized,
            vec![ProjectRelativePath::new("common/src/lib.rs")]
        );
    }

    #[test]
    fn test_normalize_change_path_candidates_keeps_existing_repo_relative_paths() {
        let normalized = normalize_change_path_candidates(
            &CellInfo::testing(),
            "jupiter/callisto",
            &workspace_root(),
            &workspace_root(),
            &HashSet::new(),
            &ProjectRelativePath::new("common/src/lib.rs"),
        );

        assert_eq!(
            normalized,
            vec![ProjectRelativePath::new("common/src/lib.rs")]
        );
    }

    #[test]
    fn test_select_change_path_candidates_keeps_ambiguous_paths_as_both_candidates() {
        let normalized = select_change_path_candidates(
            "src/access_token.rs",
            "jupiter/callisto/src/access_token.rs",
            true,
            true,
        );

        assert_eq!(
            normalized,
            vec![
                ProjectRelativePath::new("src/access_token.rs"),
                ProjectRelativePath::new("jupiter/callisto/src/access_token.rs"),
            ]
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_build_targets_detects_real_subproject_source_change() {
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
            subproject_relative,
            vec![Status::Modified(ProjectRelativePath::new(
                "src/access_token.rs",
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
            subproject_relative,
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
    async fn test_get_build_targets_handles_mixed_subproject_and_repo_relative_changes() {
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
            subproject_relative,
            vec![
                Status::Modified(ProjectRelativePath::new("README.md")),
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
}
