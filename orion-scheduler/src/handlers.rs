use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Json,
        sse::{Event, Sse},
    },
};
use serde::{Deserialize, Serialize};
use tokio::time::interval;

use crate::{orion_deployer, state::AppState, vm_cleanup};

/// Image parameters that can be passed via webhook API to override config-based image selection.
#[derive(Debug, Clone, Default)]
pub struct ImageParams {
    pub path: Option<String>,
    pub url: Option<String>,
    pub digest: Option<String>,
    pub disk_gb: Option<u32>,
    pub cpus: Option<u32>,
    pub memory_mb: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub status: String,
    pub vm_id: Option<String>,
    pub error: Option<String>,
    /// Path to the log file (not the contents)
    pub orion_log_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GithubWebhookPayload {
    pub action: Option<String>,
    /// Target environment: "aws-gitmega", "aws-gitmono", "gcp-buck2hub" (required)
    pub target: String,
    /// Override image path (local qcow2 file). Overrides default_image from config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    /// Override image URL (remote HTTPS). Overrides default_image from config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// SHA256/SHA512 digest for the image (required when image_path or image_url is set).
    /// Format: "sha256:..." or "sha512:..."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_digest: Option<String>,
    /// VM disk size in GB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_disk_gb: Option<u32>,
    /// Number of vCPUs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_cpus: Option<u32>,
    /// VM memory in MB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_memory_mb: Option<u32>,
}

/// GET /webhook
pub async fn webhook_get_handler() -> Json<WebhookResponse> {
    Json(WebhookResponse {
        status: "ok".to_string(),
        vm_id: None,
        error: None,
        orion_log_file: None,
    })
}

/// POST /webhook - receives update requests from GitHub Actions
pub async fn webhook_post_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GithubWebhookPayload>,
) -> impl IntoResponse {
    tracing::info!(
        "Received webhook: action={:?}, target={}",
        payload.action,
        payload.target
    );

    let image_params = ImageParams {
        path: payload.image_path.clone(),
        url: payload.image_url.clone(),
        digest: payload.image_digest.clone(),
        disk_gb: payload.image_disk_gb,
        cpus: payload.image_cpus,
        memory_mb: payload.image_memory_mb,
    };

    // Spawn the VM operation in a blocking task
    let target = payload.target.clone();
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        // Use blocking synchronous call since VM operations are CPU-heavy
        let rt = tokio::runtime::Handle::current();
        rt.block_on(orion_deployer::handle_update(
            &state_clone,
            &target,
            Some(image_params),
        ))
    })
    .await;

    match result {
        Ok(Ok(_vm_id)) => {
            tracing::info!("Successfully created VM: {}", _vm_id);
            // Fetch the stored VmInfo from state — it contains the log file path
            // set by handle_update before it returned.
            let orion_log_file = state.get_vm().await.and_then(|vm| vm.log_file);
            let response = WebhookResponse {
                status: "ok".to_string(),
                vm_id: Some(_vm_id),
                error: None,
                orion_log_file,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to handle update: {:?}", e);
            let response = WebhookResponse {
                status: "error".to_string(),
                vm_id: None,
                error: Some(e.to_string()),
                orion_log_file: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("Task join error: {:?}", e);
            let response = WebhookResponse {
                status: "error".to_string(),
                vm_id: None,
                error: Some(e.to_string()),
                orion_log_file: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /health
pub async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "orion-scheduler"
    }))
}

/// GET /status
pub async fn status_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match orion_deployer::get_status(&state).await {
        Some(vm) => Json(serde_json::json!({
            "status": "running",
            "vm_id": vm.id,
            "vm_ip": vm.ip,
            "uptime_secs": vm.created_at.elapsed().as_secs(),
            "log_file": vm.log_file
        })),
        None => Json(serde_json::json!({
            "status": "no_vm",
            "vm_id": null
        })),
    }
}

/// Format a single log line with colors based on content type
fn format_log_line(line: &str) -> String {
    // Remove ANSI escape codes for clean formatting
    let clean_line = strip_ansi(line);

    // Determine line type and color
    if clean_line.contains("preflight.sh") || clean_line.contains("预检") {
        format!("  🔍 {}", colorize(&clean_line, "cyan"))
    } else if clean_line.contains("cleanup.sh") || clean_line.contains("清理") {
        format!("  🧹 {}", colorize(&clean_line, "yellow"))
    } else if clean_line.contains("systemd") || clean_line.contains("Started") {
        format!("  ✅ {}", colorize(&clean_line, "green"))
    } else if clean_line.contains("ORION_WORKER_ID") || clean_line.contains("Worker ID") {
        format!("  🆔 {}", colorize(&clean_line, "magenta"))
    } else if clean_line.contains("WebSocket") || clean_line.contains("Connecting") {
        format!("  🌐 {}", colorize(&clean_line, "blue"))
    } else if clean_line.contains("Antares") || clean_line.contains("Dicfuse") {
        format!("  📦 {}", colorize(&clean_line, "bright_blue"))
    } else if clean_line.contains("ERROR") || clean_line.contains("error") {
        format!("  ❌ {}", colorize(&clean_line, "red"))
    } else if clean_line.contains("WARN") || clean_line.contains("warn") {
        format!("  ⚠️  {}", colorize(&clean_line, "yellow"))
    } else if clean_line.contains("INFO") || clean_line.contains("info") {
        format!("  ℹ️  {}", colorize(&clean_line, "white"))
    } else if clean_line.starts_with("==>") {
        format!("  ▶️  {}", colorize(&clean_line, "bright_white"))
    } else if clean_line.contains("DEBUG") {
        format!("  🔧 {}", colorize(&clean_line, "dim"))
    } else if clean_line.is_empty() {
        "  ".to_string()
    } else {
        format!("  │  {}", clean_line)
    }
}

/// Apply ANSI color code to text
/// Colors: red, green, yellow, blue, magenta, cyan, white, bright_white, bright_blue, dim
fn colorize(text: &str, color: &str) -> String {
    let code = match color {
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "bright_white" => "97",
        "bright_blue" => "94",
        "dim" => "90",
        _ => "37",
    };
    format!("\x1b[{}m{}\x1b[0m", code, text)
}

/// Remove ANSI escape sequences (color codes) from text for clean formatting
fn strip_ansi(text: &str) -> String {
    let mut result = String::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Skip until end of ANSI sequence
            i += 2;
            while i < chars.len() && !chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            i += 1; // Skip the final letter
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// GET /scorpio/status - Check Scorpio mount status and directories
pub async fn scorpio_status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match orion_deployer::get_scorpio_status(&state).await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => {
            let response = serde_json::json!({
                "status": "error",
                "error": e.to_string()
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// GET /scorpio/config - Read scorpio.toml content from VM
pub async fn scorpio_config_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let machine = match state.get_machine().await {
        Some(m) => m,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "error": "No VM is currently running"
                })),
            )
                .into_response();
        }
    };

    match machine
        .exec("cat /home/orion/orion-runner/scorpio.toml")
        .await
    {
        Ok(output) => {
            let content = String::from_utf8_lossy(&output.stdout).to_string();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ok",
                    "path": "/home/orion/orion-runner/scorpio.toml",
                    "content": content
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// POST /shutdown - Shutdown VM only, server keeps running
pub async fn shutdown_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::info!("[http-shutdown] Received shutdown request via HTTP");

    // Serialize with `handle_update`. Without this guard, /shutdown can
    // run between an in-flight create's `KeepAliveMachine::new` and its
    // `state.set_vm`, see an empty state, return success, and leave the
    // freshly-spawned qemu untracked once /webhook publishes it.
    let _update_guard = state.lock_update().await;

    if let Some(machine) = state.get_machine().await {
        tracing::info!("[http-shutdown] VM found, calling shutdown...");
        match machine.shutdown().await {
            Ok(_) => tracing::info!("[http-shutdown] VM shutdown completed successfully"),
            Err(e) => tracing::error!("[http-shutdown] VM shutdown failed: {}", e),
        }
    } else {
        tracing::info!("[http-shutdown] No VM running");
    }
    state.clear_vm().await;

    // Belt-and-suspenders: reap any orphan qemu processes that may have
    // escaped tracking (e.g. spawned by a previous crashed run, or
    // mid-init when a prior shutdown raced). Cheap and idempotent.
    let _ = tokio::process::Command::new("pkill")
        .args(["-9", "-f", "qemu-system-x86"])
        .output()
        .await;

    // Disk-side cleanup: qlean only removes the run dir from `Machine::drop`,
    // which doesn't run on SIGKILL/abort. Sweep any orphaned overlay/seed
    // files so /shutdown actually frees the VM's disk footprint, not just
    // its processes.
    vm_cleanup::sweep_stale_runs().await;

    let response = serde_json::json!({
        "status": "ok",
        "message": "VM stopped, server is still running"
    });
    (StatusCode::OK, Json(response)).into_response()
}

/// Number of trailing lines to send to the client on the first SSE tick.
const INITIAL_TAIL_LINES: usize = 50;

/// Number of trailing line hashes to remember as a content fingerprint for
/// resuming after sliding-window fetches like `journalctl -n N` / `tail -N`.
/// A longer fingerprint better disambiguates against periodic repeats
/// (heartbeats, idle pings); 10 lines comfortably exceeds typical repeat runs.
const RESUME_FINGERPRINT_LINES: usize = 10;

/// Cursor that tracks the trailing content of one log section so we can
/// resume after the next fetch without re-emitting already-streamed lines.
///
/// The data source (`journalctl -n 100`, `tail -100 ...`) returns a sliding
/// window of the most recent lines, NOT an append-only stream, so position-
/// based cursors are unsafe: as new lines arrive, the entire window shifts
/// and any "line at index N" identity is lost. Instead we record a hash
/// fingerprint of the last few lines we saw, then on the next tick locate
/// that fingerprint inside the new window and emit only what follows it.
#[derive(Default)]
struct LogCursor {
    /// Hashes of the last `RESUME_FINGERPRINT_LINES` lines from the previous
    /// fetch (oldest first). Empty before the first non-empty fetch.
    fingerprint: Vec<u64>,
}

impl LogCursor {
    /// Return the slice of `lines` that is new since the last call and
    /// advance the fingerprint to the current tail.
    fn advance<'a>(&mut self, lines: &'a [&'a str]) -> &'a [&'a str] {
        if lines.is_empty() {
            return lines;
        }
        let start = if self.fingerprint.is_empty() {
            // First non-empty fetch: show recent activity without spamming.
            lines.len().saturating_sub(INITIAL_TAIL_LINES)
        } else {
            // Resume right after the previous tail. If the source rolled past our
            // fingerprint (burst faster than the poll window), emit a recent tail
            // so the stream stays live instead of going silent until the burst ends.
            self.find_resume_index(lines)
                .unwrap_or_else(|| lines.len().saturating_sub(INITIAL_TAIL_LINES))
        };

        self.refresh_fingerprint(lines);
        &lines[start.min(lines.len())..]
    }

    /// Locate the index in `lines` immediately after the previously-seen
    /// trailing window. Tries the longest fingerprint suffix first so that
    /// when the source produces repeated identical lines (e.g. heartbeats),
    /// surrounding context disambiguates which occurrence is "ours".
    fn find_resume_index(&self, lines: &[&str]) -> Option<usize> {
        let line_hashes: Vec<u64> = lines.iter().map(|l| hash_line(l)).collect();
        let k = self.fingerprint.len();
        for window in (1..=k).rev() {
            let fp_suffix = &self.fingerprint[k - window..];
            for end in (window..=line_hashes.len()).rev() {
                if line_hashes[end - window..end] == *fp_suffix {
                    return Some(end);
                }
            }
        }
        None
    }

    fn refresh_fingerprint(&mut self, lines: &[&str]) {
        self.fingerprint.clear();
        let start = lines.len().saturating_sub(RESUME_FINGERPRINT_LINES);
        self.fingerprint
            .extend(lines[start..].iter().map(|l| hash_line(l)));
    }
}

fn hash_line(line: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    line.hash(&mut hasher);
    hasher.finish()
}

/// GET /logs/orion/stream - SSE stream for real-time log viewing.
/// First tick sends the last `INITIAL_TAIL_LINES` lines, then only newly
/// appended lines on each subsequent tick.
pub async fn logs_stream_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let stream = async_stream::stream! {
        let mut ticker = interval(std::time::Duration::from_secs(1));
        let mut journal_cursor = LogCursor::default();
        let mut orion_log_offset: u64 = 0;

        loop {
            ticker.tick().await;

            let snapshot = match orion_deployer::get_live_logs_since(&state, orion_log_offset).await {
                Ok(snapshot) => snapshot,
                Err(e) => {
                    yield Ok(Event::default().data(format!("Error: {}", e)));
                    continue;
                }
            };
            orion_log_offset = snapshot.orion_log_offset;

            let journal_lines: Vec<&str> = snapshot.journal_window.lines().collect();
            let new_j = journal_cursor.advance(&journal_lines);
            let orion_lines: Vec<&str> = snapshot.orion_log_delta.lines().collect();

            if new_j.is_empty() && orion_lines.is_empty() {
                continue;
            }

            let mut output = String::new();
            if !new_j.is_empty() {
                append_logs_section(&mut output, "SYSTEM LOGS", new_j);
            }
            if !orion_lines.is_empty() {
                append_logs_section(&mut output, "ORION LOGS", &orion_lines);
            }

            yield Ok(Event::default().comment("---").data(output));
        }
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

/// Append a log section with a title header and colored log lines to `output`.
fn append_logs_section(output: &mut String, title: &str, lines: &[&str]) {
    use std::fmt::Write;
    let _ = writeln!(output, "\n─── {} ───", title);
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        output.push_str(&format_log_line(trimmed));
        output.push('\n');
    }
}
