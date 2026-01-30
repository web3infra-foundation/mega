use std::{fs, path::Path, process::Command, time::Duration};

use once_cell::sync::OnceCell;
use tokio_retry::{Retry, strategy::ExponentialBackoff};
use uuid::Uuid;

/// Mono base URL for file/blob API, set at startup from config (or default).
static MONO_BASE_URL: OnceCell<String> = OnceCell::new();

/// Set the mono base URL from config. Call once at server startup.
pub fn set_mono_base_url(url: String) {
    let _ = MONO_BASE_URL.set(url);
}

/// Download files from file blob API using two hash values and save them to a new folder in tmp directory
async fn download_files_to_tmp(
    hash1: &str,
    hash2: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create tmp directory path
    // Generate a unique folder name using UUID
    let folder_name = Uuid::now_v7().to_string();
    println!("Generated folder name: {folder_name},buck:{hash1},.buckconfig:{hash2}");

    // Create tmp directory path
    let tmp_dir = std::env::temp_dir().join(folder_name);

    fs::create_dir_all(&tmp_dir)?;

    // Download first file as BUCK
    let buck_path = tmp_dir.join("BUCK");
    download_file_with_retry(hash1, &buck_path, 3).await?;

    // Download second file as .buckconfig
    let buckconfig_path = tmp_dir.join(".buckconfig");
    download_file_with_retry(hash2, &buckconfig_path, 3).await?;

    Ok(tmp_dir.to_string_lossy().to_string())
}

/// Download a single file with retry mechanism using tokio-retry
async fn download_file_with_retry(
    hash: &str,
    file_path: &Path,
    max_retries: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_endpoint = file_blob_endpoint();
    let url = format!("{api_endpoint}/{hash}");

    // Create retry strategy: exponential backoff starting from 100ms, max 3 attempts
    let retry_strategy = ExponentialBackoff::from_millis(100)
        .max_delay(Duration::from_secs(2))
        .take(max_retries);

    Retry::spawn(retry_strategy, || download_single_file(&url, file_path))
        .await
        .map_err(|e| format!("Failed to download {hash} after {max_retries} attempts: {e}").into())
}

/// Download a single file from URL and save to specified path
async fn download_single_file(
    url: &str,
    file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    let content = response.bytes().await?;
    fs::write(file_path, &content)?;

    Ok(())
}

/// Get the base URL for API requests (from config, or default).
fn base_url() -> String {
    MONO_BASE_URL
        .get()
        .cloned()
        .unwrap_or_else(|| "http://localhost:8000".to_string())
}

/// Get the file blob API endpoint
pub fn file_blob_endpoint() -> String {
    format!("{}/api/v1/file/blob", base_url())
}

/// Execute buck2 targets //... command in the given directory and return the last line string
#[allow(dead_code)]
pub fn get_buck2_targets_last_line(directory: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("buck2")
        .args(["targets", "//..."])
        .current_dir(directory)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("buck2 command failed: {stderr}").into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let last_line = stdout.lines().last().unwrap_or("").to_string();

    Ok(last_line)
}

/// Download files and execute buck2 targets command to get the last line output
#[allow(dead_code)]
pub async fn download_and_get_buck2_targets(
    hash1: &str,
    hash2: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // First, download the files (BUCK and .buckconfig) to tmp directory
    let tmp_dir_path = download_files_to_tmp(hash1, hash2).await?;

    // Then, execute buck2 targets command in the downloaded directory
    let last_line = get_buck2_targets_last_line(&tmp_dir_path)?;

    // Clean up the temporary directory after getting the result
    //TODO: Cleanup should happen in a finally block or using RAII to ensure temporary files are removed even if buck2 command fails. Consider using a guard or defer pattern. TempDirGuard , for example.
    if Path::new(&tmp_dir_path).exists() {
        fs::remove_dir_all(&tmp_dir_path)?;
    }

    Ok(last_line)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn test_get_buck2_targets_last_line() {
        // Use the test directory within the project
        let test_dir = "./test";
        if Path::new(test_dir).exists() {
            // Create a temporary directory for testing
            let tmp_test_dir = std::env::temp_dir().join("buck2_test");

            // Remove existing tmp directory if it exists
            if tmp_test_dir.exists() {
                fs::remove_dir_all(&tmp_test_dir).expect("Failed to remove existing tmp directory");
            }

            // Copy test directory contents to tmp directory
            copy_dir_all(test_dir, &tmp_test_dir).expect("Failed to copy test directory to tmp");

            // Run buck2 targets command in the tmp directory
            match get_buck2_targets_last_line(tmp_test_dir.to_str().unwrap()) {
                Ok(last_line) => {
                    println!("Last line: {last_line}");
                    assert!(!last_line.is_empty());
                }
                Err(e) => println!("Warning: {e}"),
            }

            // Clean up: remove the tmp directory after test
            if tmp_test_dir.exists() {
                fs::remove_dir_all(&tmp_test_dir).expect("Failed to clean up tmp directory");
            }
        } else {
            println!("Test directory '{test_dir}' does not exist. Skipping test.");
        }
    }

    /// Helper function to recursively copy directory contents
    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }
}
