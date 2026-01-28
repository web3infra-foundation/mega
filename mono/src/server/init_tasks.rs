use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use tokio::time::sleep;

const GIT_USER_EMAIL: &str = "mega-bot@example.com";
const GIT_USER_NAME: &str = "Mega Bot";
const BUCKAL_BUNDLES_REPO: &str = "https://github.com/buck2hub/buckal-bundles.git";
const LIBRA_REPO: &str = "https://github.com/web3infra-foundation/libra.git";
const COMMIT_MSG: &str = "import buckal-bundles";
// Relative path from project root
const IMPORT_SCRIPT_PATH: &str = "scripts/import-buck2-deps/import-buck2-deps.py";

/// Runs the server initialization tasks asynchronously.
///
/// This function spawns a blocking task to run workflows that depend on the server being up and running.
/// It waits for a short duration to ensure the server has started before kicking off the workflows.
///
/// # Arguments
///
/// * `host` - The host address the server is listening on.
/// * `port` - The port number the server is listening on.
pub async fn run_initialization_tasks(host: String, port: u16) {
    tracing::info!("Initialization tasks scheduled. Waiting 5s for server startup...");
    sleep(Duration::from_secs(5)).await;

    let target_host = if host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        host
    };
    let base_url = format!("http://{}:{}", target_host, port);

    // Run blocking tasks in a blocking thread to avoid blocking the async runtime
    let _ = tokio::task::spawn_blocking(move || {
        tracing::info!("Starting initialization workflows against {}", base_url);

        if let Err(e) = run_buckal_bundles_workflow(&base_url) {
            tracing::error!("Buckal Bundles workflow failed: {:?}", e);
        } else {
            tracing::info!("Buckal Bundles workflow completed.");
        }

        if let Err(e) = run_libra_workflow(&base_url) {
            tracing::error!("Libra workflow failed: {:?}", e);
        } else {
            tracing::info!("Libra workflow completed.");
        }
    })
    .await;
}

/// Executes the workflow to import buckal-bundles.
///
/// This involves cloning the toolchains repo, importing the buckal-bundles repo into it,
/// committing the changes, and creating/merging a Change List (CL).
///
/// # Arguments
///
/// * `base_url` - The base URL of the Mega server.
fn run_buckal_bundles_workflow(base_url: &str) -> anyhow::Result<()> {
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("mega-init-buckal-{}", timestamp));
    std::fs::create_dir_all(&temp_dir)?;

    // Use a closure to ensure cleanup happens even if errors occur
    let result = (|| {
        let toolchains_dir = prepare_toolchains_repo(base_url, &temp_dir)?;

        import_buckal_bundles(&toolchains_dir)?;

        commit_and_push(&toolchains_dir, COMMIT_MSG)?;

        handle_merge_request(base_url, COMMIT_MSG)
    })();

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    result
}

/// Prepares the toolchains repository by cloning it and configuring the git user.
///
/// # Arguments
///
/// * `base_url` - The base URL of the Mega server.
/// * `temp_dir` - The temporary directory where the repo should be cloned.
///
/// # Returns
///
/// The path to the cloned toolchains directory.
fn prepare_toolchains_repo(base_url: &str, temp_dir: &Path) -> anyhow::Result<PathBuf> {
    tracing::info!("Cloning toolchains to {:?}", temp_dir);
    let toolchains_url = format!("{}/toolchains.git", base_url);

    run_git(temp_dir, &["clone", &toolchains_url])?;

    let toolchains_dir = temp_dir.join("toolchains");

    // Configure git user
    run_git(&toolchains_dir, &["config", "user.email", GIT_USER_EMAIL])?;
    run_git(&toolchains_dir, &["config", "user.name", GIT_USER_NAME])?;

    Ok(toolchains_dir)
}

/// Imports the buckal-bundles repository into the toolchains directory.
///
/// This clones the buckal-bundles repo and removes its .git directory to treat it as a submodule/vendored code.
///
/// # Arguments
///
/// * `toolchains_dir` - The path to the toolchains directory.
fn import_buckal_bundles(toolchains_dir: &Path) -> anyhow::Result<()> {
    tracing::info!("Cloning buckal-bundles inside toolchains...");
    run_git(
        toolchains_dir,
        &["clone", "--depth", "1", BUCKAL_BUNDLES_REPO],
    )?;

    // remove buckal-bundles/.git file
    let buckal_git_dir = toolchains_dir.join("buckal-bundles").join(".git");
    if buckal_git_dir.exists() {
        std::fs::remove_dir_all(&buckal_git_dir)?;
    }
    Ok(())
}

/// Commits all changes in the directory and pushes them to the remote.
///
/// # Arguments
///
/// * `dir` - The directory containing the git repository.
/// * `msg` - The commit message.
fn commit_and_push(dir: &Path, msg: &str) -> anyhow::Result<()> {
    tracing::info!("Committing and pushing changes...");
    run_git(dir, &["add", "."])?;
    run_git(dir, &["commit", "-m", msg])?;
    run_git(dir, &["push"])?;
    Ok(())
}

/// Handles the creation and merging of a Change List (CL) for the imported changes.
///
/// # Arguments
///
/// * `base_url` - The base URL of the Mega server.
/// * `title` - The title of the CL to look for.
fn handle_merge_request(base_url: &str, title: &str) -> anyhow::Result<()> {
    tracing::info!("Listing CLs to find merge request...");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;

    wait_http_ready(&client, base_url, Duration::from_secs(30))?;

    let link = find_cl_link_with_retry(&client, base_url, title, Duration::from_secs(90))?;
    tracing::info!("Found CL with link: {}", link);

    merge_cl_with_retry(&client, base_url, &link, Duration::from_secs(60))?;
    tracing::info!("Merge completed successfully: {}", link);
    Ok(())
}

/// Executes the workflow to import libra dependencies.
///
/// This involves cloning the libra repo and running a python script to import dependencies.
///
/// # Arguments
///
/// * `base_url` - The base URL of the Mega server.
fn run_libra_workflow(base_url: &str) -> anyhow::Result<()> {
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("mega-init-libra-{}", timestamp));
    std::fs::create_dir_all(&temp_dir)?;

    let result = (|| {
        tracing::info!("Cloning libra to {:?}", temp_dir);
        run_git(&temp_dir, &["clone", LIBRA_REPO, "."])?;

        let script_path = std::env::current_dir()?.join(IMPORT_SCRIPT_PATH);
        if !script_path.exists() {
            return Err(anyhow::anyhow!(
                "Import script not found at {:?}",
                script_path
            ));
        }

        let third_party_path = temp_dir.join("third-party");

        tracing::info!("Running import script for libra...");
        let status = Command::new("python3")
            .arg(script_path)
            .arg("--scan-root")
            .arg(third_party_path)
            .arg("--git-base-url")
            .arg(base_url)
            .arg("--jobs")
            .arg("8")
            .arg("--retry")
            .arg("3")
            .current_dir(&temp_dir)
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Python script failed"));
        }
        Ok(())
    })();

    let _ = std::fs::remove_dir_all(&temp_dir);
    result
}

/// Helper function to run a git command in a specific directory.
///
/// # Arguments
///
/// * `dir` - The directory to run the command in.
/// * `args` - The arguments to pass to the git command.
fn run_git(dir: &Path, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Git command failed: git {:?}", args));
    }
    Ok(())
}

/// Waits for the HTTP server to be ready by checking the status endpoint.
///
/// # Arguments
///
/// * `client` - The HTTP client to use.
/// * `base_url` - The base URL of the Mega server.
/// * `max_wait` - The maximum duration to wait.
fn wait_http_ready(
    client: &reqwest::blocking::Client,
    base_url: &str,
    max_wait: Duration,
) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    let status_url = format!("{}/api/v1/status", base_url);
    let mut last_err: Option<anyhow::Error> = None;

    while start.elapsed() < max_wait {
        match client.get(&status_url).send() {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            Ok(resp) => {
                last_err = Some(anyhow::anyhow!(
                    "status endpoint not ready: {}",
                    resp.status()
                ));
            }
            Err(e) => {
                last_err = Some(anyhow::anyhow!("status endpoint request failed: {}", e));
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("status endpoint not ready")))
}

/// Finds the Change List (CL) link for a given title with retries.
///
/// # Arguments
///
/// * `client` - The HTTP client to use.
/// * `base_url` - The base URL of the Mega server.
/// * `title` - The title of the CL to search for.
/// * `max_wait` - The maximum duration to wait/retry.
///
/// # Returns
///
/// The link identifier of the found CL.
fn find_cl_link_with_retry(
    client: &reqwest::blocking::Client,
    base_url: &str,
    title: &str,
    max_wait: Duration,
) -> anyhow::Result<String> {
    let start = std::time::Instant::now();
    let list_url = format!("{}/api/v1/cl/list", base_url);
    let mut last_err: Option<anyhow::Error> = None;

    while start.elapsed() < max_wait {
        for page in 1..=5 {
            let body = serde_json::json!({
                "pagination": {
                    "page": page,
                    "per_page": 20
                },
                "additional": {
                    "sort_by": "created_at",
                    "status": "open",
                    "asc": false
                }
            });

            let resp = client
                .post(&list_url)
                .header("accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&body)
                .send();

            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("list CLs request failed: {}", e));
                    continue;
                }
            };

            let status = resp.status();
            let text = resp.text().unwrap_or_default();

            if !status.is_success() {
                let msg = if text.is_empty() {
                    format!("list CLs returned {}", status)
                } else {
                    format!("list CLs returned {}: {}", status, truncate(&text, 300))
                };
                last_err = Some(anyhow::anyhow!(msg));
                continue;
            }

            let json: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("failed to parse CL list response: {}", e));
                    continue;
                }
            };

            if !json["req_result"].as_bool().unwrap_or(false) {
                let err_msg = json["err_message"].as_str().unwrap_or("unknown error");
                last_err = Some(anyhow::anyhow!("CL list request failed: {}", err_msg));
                continue;
            }

            if let Some(cl) = json["data"]["items"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["title"] == title))
            {
                let link = cl["link"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing link"))?;
                return Ok(link.to_string());
            }
        }

        std::thread::sleep(Duration::from_secs(2));
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("No CL found for import")))
}

/// Merges a Change List (CL) with retries.
///
/// # Arguments
///
/// * `client` - The HTTP client to use.
/// * `base_url` - The base URL of the Mega server.
/// * `link` - The link identifier of the CL to merge.
/// * `max_wait` - The maximum duration to wait/retry.
fn merge_cl_with_retry(
    client: &reqwest::blocking::Client,
    base_url: &str,
    link: &str,
    max_wait: Duration,
) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    let merge_url = format!("{}/api/v1/cl/{}/merge-no-auth", base_url, link);
    let mut last_err: Option<anyhow::Error> = None;

    while start.elapsed() < max_wait {
        let resp = client
            .post(&merge_url)
            .header("accept", "application/json")
            .body("")
            .send();

        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => {
                last_err = Some(anyhow::anyhow!("merge request failed: {}", e));
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let status = resp.status();
        let text = resp.text().unwrap_or_default();

        if !status.is_success() {
            let msg = if text.is_empty() {
                format!("merge returned {}", status)
            } else {
                format!("merge returned {}: {}", status, truncate(&text, 300))
            };
            last_err = Some(anyhow::anyhow!(msg));
            std::thread::sleep(Duration::from_secs(2));
            continue;
        }

        let json: serde_json::Value = serde_json::from_str(&text)?;
        if !json["req_result"].as_bool().unwrap_or(false) {
            let err_msg = json["err_message"].as_str().unwrap_or("unknown error");
            last_err = Some(anyhow::anyhow!("Merge failed: {}", err_msg));
            std::thread::sleep(Duration::from_secs(2));
            continue;
        }

        return Ok(());
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Merge failed")))
}

/// Truncates a string to a maximum length.
///
/// # Arguments
///
/// * `s` - The string to truncate.
/// * `max_len` - The maximum length.
///
/// # Returns
///
/// The truncated string.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len).collect::<String>()
    }
}
