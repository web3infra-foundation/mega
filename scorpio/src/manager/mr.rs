use std::path::PathBuf;

use reqwest::Client;
use serde::Deserialize;

use crate::util::config;

/// Single file record
#[derive(Debug, Deserialize)]
struct FileInfo {
    action: String,
    path:   String,
    _sha:    String,
}

/// Response body for /files-list endpoint
#[derive(Debug, Deserialize)]
struct FilesListResp {
    data:        Vec<FileInfo>,
    err_message: String,
    req_result:  bool,
}

pub async fn build_mr_layer(
    link: &str,
    _mr_path:PathBuf
) -> Result<(), reqwest::Error> {
    let files_list = fetch_files_list(link).await?;

    if !files_list.req_result {
        println!("{}", files_list.err_message);
        return Ok(());
    }

    for file in files_list.data {
        if file.action == "remove" {
            // ! ! ! Attention:
            // As for deleted files, the path should create a `whiteout file`, refer to create_whiteout in other files .
            println!("{}", file.path);
        }
    }
    //TODO: Implement the logic to build the MR layer
    // This could involve checking out the MR branch, applying changes, etc.
    // such as fetch::fetch_by_hashs(&files_list.data, &mr_path)
   
    println!("MR layer built for link: {link}");
    Ok(())
}

/// Fetch the list of files for a Merge Request (MR)
///
/// - `link`: unique identifier for the MR (used in the path)
///
/// Returns `FilesListResp` on success, or `reqwest::Error` on failure.
async fn fetch_files_list(
    link: &str,
) -> Result<FilesListResp, reqwest::Error> {
    let url = format!(
        "{}/api/v1/mr/{}/files-list",
        config::base_url(),
        link
    );

    Client::new()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<FilesListResp>()
        .await
}
