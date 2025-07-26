use std::path::PathBuf;

use reqwest::Client;
use serde::Deserialize;

use crate::manager::fetch::download_mr_files;
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

pub async fn build_mr_layer(
    link: &str,
    mr_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let files_list = fetch_files_list(link).await?;

    if !files_list.req_result {
        println!("{}", files_list.err_message);
        return Ok(());
    }
    if !mr_path.exists() {
        std::fs::create_dir_all(&mr_path)?;
    }

    // Collect all files that need to be downloaded
    let mut download_files = Vec::new();
    for file in files_list.data {
        let relative_path = file.path.strip_prefix('/').unwrap_or(&file.path);
        let file_path = mr_path.join(relative_path);

        match file.action.as_str() {
            "new" | "modified" => {
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                // Parse SHA1 from string and add to download queue
                let file_id = file.sha.parse().expect("Invalid SHA1 format");
                download_files.push((file_id, file_path));
            }
            "deleted" => {
                // Create whiteout file for deleted files in overlay filesystem
                // Whiteout files are character devices with 0/0 device number
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                    let whiteout_path = parent.join(file_path.file_name().unwrap());

                    // Create whiteout as character device with 0/0 device number
                    // This is the standard way overlay filesystems recognize deleted files
                    let mode = libc::S_IFCHR | 0o777;
                    let dev = libc::makedev(0, 0);

                    let whiteout_path_cstr =
                        std::ffi::CString::new(whiteout_path.to_string_lossy().as_bytes())?;

                    let result = unsafe { libc::mknod(whiteout_path_cstr.as_ptr(), mode, dev) };

                    if result != 0 {
                        let err = std::io::Error::last_os_error();
                        eprintln!("Failed to create whiteout file {}: {}", file.path, err);
                    }
                }

                // Reserved for future use when overlay mount supports recognizing `.wh.filename` files
                // if let Some(parent) = file_path.parent() {
                //     std::fs::create_dir_all(parent)?;
                //     let whiteout_name =
                //         format!(".wh.{}", file_path.file_name().unwrap().to_string_lossy());
                //     let whiteout_path = parent.join(whiteout_name);
                //     std::fs::File::create(whiteout_path)?;
                // }
            }
            _ => {
                println!("Unknown action: {}", file.action);
            }
        }
    }

    // Download all files concurrently
    if !download_files.is_empty() {
        download_mr_files(download_files).await?;
    }

    println!("MR layer built for link: {link}");
    Ok(())
}

/// Fetch the list of files for a Merge Request (MR)
///
/// - `link`: unique identifier for the MR (used in the path)
///
/// Returns `FilesListResp` on success, or `reqwest::Error` on failure.
async fn fetch_files_list(link: &str) -> Result<FilesListResp, reqwest::Error> {
    let url = format!("{}/api/v1/mr/{}/files-list", config::base_url(), link);

    Client::new()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<FilesListResp>()
        .await
}
