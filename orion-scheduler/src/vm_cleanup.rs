//! Disk-side cleanup for leaked qlean run directories.
//!
//! qlean stores per-VM runtime state under `~/.local/share/qlean/runs/<id>/`
//! (overlay.img, seed.iso, qemu.pid, cid). `qlean::Machine::drop` removes the
//! directory only when the wrapper is dropped normally — anything that bypasses
//! unwinding (SIGKILL, panic=abort, OOM, a `handle_update` aborted before
//! `KeepAliveMachine` ever made it into state) leaks the whole directory.
//! Each leak is roughly 0.5–3 GB and we have seen them accumulate to tens of
//! gigabytes in practice.
//!
//! This module sweeps directories whose `qemu.pid` no longer maps to a live
//! process. It is safe to run at startup (after we pkill any stale qemu) and
//! at shutdown (after the same pkill), and it is idempotent.

use std::path::{Path, PathBuf};

/// Locate the qlean `runs/` directory the same way `directories::ProjectDirs`
/// does on Linux — qlean uses `ProjectDirs::from("", "", "qlean")`. We can't
/// reuse qlean's helper because it's `pub(crate)`, so we mirror the lookup
/// here instead of pulling in a new crate.
fn qlean_runs_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => {
            let home = std::env::var_os("HOME")?;
            PathBuf::from(home).join(".local/share")
        }
    };
    Some(base.join("qlean").join("runs"))
}

/// Remove every qlean run directory whose recorded qemu pid is no longer a
/// live process. Returns the number of directories reclaimed.
pub async fn sweep_stale_runs() -> usize {
    let Some(runs_dir) = qlean_runs_dir() else {
        tracing::warn!("[sweep] cannot resolve qlean runs dir; HOME unset");
        return 0;
    };

    let mut entries = match tokio::fs::read_dir(&runs_dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return 0,
        Err(e) => {
            tracing::warn!("[sweep] read_dir {} failed: {e}", runs_dir.display());
            return 0;
        }
    };

    let mut reaped = 0usize;
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(e) => {
                tracing::warn!("[sweep] read_dir iteration failed: {e}");
                break;
            }
        };

        let path = entry.path();
        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
        if !is_dir {
            continue;
        }

        if is_run_alive(&path).await {
            tracing::debug!("[sweep] keeping live run {}", path.display());
            continue;
        }

        match tokio::fs::remove_dir_all(&path).await {
            Ok(()) => {
                tracing::info!("[sweep] reclaimed stale run dir {}", path.display());
                reaped += 1;
            }
            Err(e) => tracing::warn!("[sweep] remove {} failed: {e}", path.display()),
        }
    }

    if reaped > 0 {
        tracing::warn!("[sweep] reclaimed {reaped} stale qlean run dir(s)");
    }
    reaped
}

/// A run directory is considered alive when its `qemu.pid` exists, parses,
/// and refers to a process currently present in `/proc`. Anything else
/// (missing pid file, malformed pid, dead pid) is treated as a leak.
async fn is_run_alive(run_dir: &Path) -> bool {
    let pid_file = run_dir.join("qemu.pid");
    let Ok(contents) = tokio::fs::read_to_string(&pid_file).await else {
        return false;
    };
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return false;
    };
    Path::new(&format!("/proc/{pid}")).exists()
}
