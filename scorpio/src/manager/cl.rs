use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::Deserialize;

use crate::manager::fetch::download_cl_files;
use crate::util::config;

/// Single file record
#[derive(Debug, Deserialize)]
struct FileInfo {
    action: String,
    path: String,
    sha: String,
}

/// Response body for /files-list endpoint
#[derive(Debug, Deserialize)]
struct FilesListResp {
    data: Vec<FileInfo>,
    err_message: String,
    req_result: bool,
}

/// Error type for CL layer operations
#[derive(Debug)]
pub enum ClLayerError {
    /// Failed to fetch files list from server
    FetchError(String),
    /// Server returned an error response
    ServerError(String),
    /// Failed to parse file SHA
    InvalidSha(String),
    /// Failed to create directory
    DirectoryError(std::io::Error),
    /// Failed to download files
    DownloadError(String),
    /// Failed to create whiteout file
    WhiteoutError { path: String, error: std::io::Error },
    /// Invalid or unsafe path returned by server (e.g., path traversal)
    InvalidPath(String),
}

impl std::fmt::Display for ClLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClLayerError::FetchError(msg) => write!(f, "Failed to fetch CL files list: {}", msg),
            ClLayerError::ServerError(msg) => write!(f, "Server error: {}", msg),
            ClLayerError::InvalidSha(sha) => write!(f, "Invalid SHA format: {}", sha),
            ClLayerError::DirectoryError(e) => write!(f, "Failed to create directory: {}", e),
            ClLayerError::DownloadError(msg) => write!(f, "Failed to download files: {}", msg),
            ClLayerError::WhiteoutError { path, error } => {
                write!(f, "Failed to create whiteout for {}: {}", path, error)
            }
            ClLayerError::InvalidPath(path) => {
                write!(f, "Invalid/unsafe file path returned by server: {}", path)
            }
        }
    }
}

impl std::error::Error for ClLayerError {}

/// Build a CL (Change List) layer for overlay filesystem.
///
/// This function fetches the list of changed files from the server and:
/// - Downloads new/modified files to the CL directory
/// - Creates whiteout files for deleted files (overlay filesystem convention)
///
/// # Arguments
/// * `link` - Unique identifier for the CL
/// * `cl_path` - Directory where CL layer files will be stored
/// * `repo_path` - Repository path to filter files (only files under this path are processed)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ClLayerError)` on failure with specific error type
pub async fn build_cl_layer(
    link: &str,
    cl_path: PathBuf,
    repo_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        "Building CL layer for link: {}, repo_path: {}",
        link,
        repo_path
    );

    let files_list = fetch_files_list(link)
        .await
        .map_err(|e| ClLayerError::FetchError(e.to_string()))?;

    // Check server response status - return error instead of silently succeeding
    if !files_list.req_result {
        tracing::error!(
            "Server returned error for CL {}: {}",
            link,
            files_list.err_message
        );
        return Err(Box::new(ClLayerError::ServerError(files_list.err_message)));
    }

    if files_list.data.is_empty() {
        tracing::info!("No files in CL {}, nothing to do", link);
        return Ok(());
    }

    // Create CL directory if it doesn't exist
    if !cl_path.exists() {
        std::fs::create_dir_all(&cl_path).map_err(ClLayerError::DirectoryError)?;
    }

    // Collect all files that need to be downloaded
    let mut download_files = Vec::new();
    let mut whiteout_errors = Vec::new();

    for file in files_list.data {
        // fetched path is absolute path, here we convert it to path repo
        let file_path_clean = file.path.trim_start_matches('/');
        let repo_path_clean = repo_path.trim_start_matches('/');

        // Skip files not under the repo path
        if !file_path_clean.starts_with(repo_path_clean) {
            tracing::debug!("Skipping file outside repo path: {}", file.path);
            continue;
        }

        // NOTE: `starts_with(repo_path_clean)` above implies strip_prefix must succeed.
        // If it doesn't, treat it as a server-side bug and fail loudly.
        let relative_path = file_path_clean
            .strip_prefix(repo_path_clean)
            .ok_or_else(|| ClLayerError::InvalidPath(file.path.clone()))?
            .trim_start_matches('/');

        // Security: validate the returned path does not escape `cl_path` (no absolute/prefix/..).
        // This prevents path traversal if the CL service is compromised or misbehaves.
        let safe_rel = sanitize_relative_path(relative_path)
            .map_err(|_| ClLayerError::InvalidPath(file.path.clone()))?;
        let file_path = cl_path.join(safe_rel);

        match file.action.as_str() {
            "new" | "modified" => {
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent).map_err(ClLayerError::DirectoryError)?;
                }
                // Parse ObjectHash from string and add to download queue
                let file_id = file
                    .sha
                    .parse()
                    .map_err(|_| ClLayerError::InvalidSha(file.sha.clone()))?;
                download_files.push((file_id, file_path));
            }
            "deleted" => {
                if let Err(e) = create_whiteout_file(&file_path) {
                    // Log error but continue processing other files
                    tracing::warn!("Failed to create whiteout for {}: {}", file.path, e);
                    whiteout_errors.push((file.path.clone(), e));
                }
            }
            action => {
                tracing::warn!("Unknown action '{}' for file: {}", action, file.path);
            }
        }
    }

    // Download all files concurrently
    if !download_files.is_empty() {
        tracing::info!("Downloading {} files for CL {}", download_files.len(), link);
        download_cl_files(download_files)
            .await
            .map_err(|e| ClLayerError::DownloadError(e.to_string()))?;
    }

    // Report whiteout errors if any (but don't fail the whole operation)
    if !whiteout_errors.is_empty() {
        tracing::warn!(
            "CL {} built with {} whiteout errors (deleted files may not be properly hidden)",
            link,
            whiteout_errors.len()
        );
    }

    tracing::info!("CL layer built successfully for link: {}", link);
    Ok(())
}

/// Create a whiteout file for overlay filesystem.
///
/// Whiteout files are used by overlay filesystems to mark files as deleted.
/// This function tries two methods:
/// 1. Character device with 0/0 device number (requires CAP_MKNOD or root)
/// 2. Fallback to .wh.<filename> empty file (works without special privileges).
///
/// NOTE: `.wh.*` is a convention used by some overlay implementations/tools. If the overlayfs
/// implementation that consumes this CL layer does NOT recognize `.wh.*`, deleted files may not
/// be hidden correctly. Prefer running with privileges so method (1) works for correctness.
fn create_whiteout_file(file_path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;

        let file_name = file_path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "No filename"))?;

        // Try method 1: Character device (standard overlay whiteout)
        // TODO(cl-layer): Improve variable naming here (e.g., `whiteout_dev_path` vs
        // `whiteout_fallback_path`) to avoid confusion between the device whiteout and `.wh.*`.
        let whiteout_path = parent.join(file_name);
        let mode = libc::S_IFCHR | 0o777;
        let dev = libc::makedev(0, 0);

        let whiteout_path_cstr = std::ffi::CString::new(whiteout_path.to_string_lossy().as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let result = unsafe { libc::mknod(whiteout_path_cstr.as_ptr(), mode, dev) };

        if result == 0 {
            return Ok(());
        }

        let mknod_err = std::io::Error::last_os_error();

        // If mknod failed due to permissions, try fallback method
        if mknod_err.raw_os_error() == Some(libc::EPERM)
            || mknod_err.raw_os_error() == Some(libc::EACCES)
        {
            tracing::debug!(
                "mknod failed for {}, using .wh. fallback: {}",
                whiteout_path.display(),
                mknod_err
            );
            tracing::warn!(
                "Using .wh.* fallback for whiteout at {} (mknod failed). Deletion masking may be \
                 incorrect if the overlay implementation does not recognize .wh.*; prefer running \
                 as root/CAP_MKNOD for correct whiteouts.",
                whiteout_path.display()
            );

            // Method 2: Create .wh.<filename> empty file (fallback for non-root)
            let whiteout_name = format!(".wh.{}", file_name.to_string_lossy());
            let whiteout_fallback_path = parent.join(whiteout_name);
            std::fs::File::create(&whiteout_fallback_path)?;
            tracing::debug!(
                "Created fallback whiteout: {}",
                whiteout_fallback_path.display()
            );
            return Ok(());
        }

        Err(mknod_err)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "File path has no parent directory",
        ))
    }
}

fn sanitize_relative_path(p: &str) -> Result<PathBuf, ()> {
    let path = Path::new(p);
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::Normal(x) => out.push(x),
            std::path::Component::CurDir => {}
            // Reject anything that can escape the directory.
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => return Err(()),
        }
    }
    Ok(out)
}

/// Fetch the list of files for a Change List (CL)
///
/// - `link`: unique identifier for the CL (used in the path)
///
/// Returns `FilesListResp` on success, or `reqwest::Error` on failure.
async fn fetch_files_list(link: &str) -> Result<FilesListResp, reqwest::Error> {
    let url = format!("{}/api/v1/cl/{}/files-list", config::base_url(), link);
    tracing::debug!("Fetching CL files list from: {}", url);

    Client::new()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<FilesListResp>()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cl_layer_error_display() {
        let err = ClLayerError::ServerError("test error".to_string());
        assert!(err.to_string().contains("Server error"));

        let err = ClLayerError::InvalidSha("bad-sha".to_string());
        assert!(err.to_string().contains("Invalid SHA"));
    }
}
