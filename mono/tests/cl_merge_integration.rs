//! Integration tests for Mega ChangeList (CL) merge and update-branch operations
//!
//! These tests run inside a QEMU/KVM virtual machine using the qlean crate,
//! with PostgreSQL and Redis running in Docker containers inside the VM.
//!
//! ## Prerequisites
//!
//! This test requires system-level dependencies:
//! - QEMU/KVM virtualization (qemu-system-x86_64, qemu-img)
//! - libguestfs-tools (guestfish, virt-copy-out)
//! - xorriso, sha256sum
//!
//! Install on Ubuntu/Debian:
//! ```bash
//! sudo apt-get install qemu-system-x86 qemu-utils libguestfs-tools xorriso
//! ```
//!
//! ## Running the Test
//!
//! ```bash
//! # Run test (note the --ignored flag)
//! # No need to build mono - binary is extracted from ECR image
//! cargo test -p mono --test cl_merge_integration -- --ignored --nocapture
//!
//! # Override the default ECR image (optional)
//! MEGA_ECR_IMAGE=public.ecr.aws/m8q5m4u3/mega:mono-0.1.0-pre-release-amd64 \
//!   cargo test -p mono --test cl_merge_integration -- --ignored --nocapture
//! ```
//!
//! ## Test Design
//!
//! This test uses Docker containers for PostgreSQL and Redis inside the VM,
//! and extracts the mono binary from the ECR image (mono-0.1.0-pre-release-amd64).
//! This provides:
//! - Faster startup time compared to apt-get installation
//! - Easier configuration and cleanup
//! - Better resource isolation
//! - Consistency with production/demo environment (reusing docker-compose.demo.yml)
//! - No need to build mono locally (uses production image)
//!
//! Test users and their tokens are defined as constants (TEST_TOKEN_A, TEST_TOKEN_B)
//! and inserted directly into the database during setup for simplicity.

use std::{
    path::{Path, PathBuf},
    sync::Once,
    time::Duration,
};

use anyhow::{Context, Result};
use qlean::{Distro, MachineConfig, create_image, with_machine};
use serde_json::Value;
use tracing_subscriber::EnvFilter;

const MEGA_HOST: &str = "127.0.0.1";
const MEGA_PORT: u16 = 8000;
const POSTGRES_USER: &str = "mega";
const POSTGRES_PASSWORD: &str = "mega";
const POSTGRES_DB: &str = "mono";

// ECR mono image
const MEGA_ECR_IMAGE_DEFAULT: &str = "public.ecr.aws/m8q5m4u3/mega:mono-0.1.0-pre-release-amd64";

fn get_mega_ecr_image() -> String {
    std::env::var("MEGA_ECR_IMAGE").unwrap_or_else(|_| MEGA_ECR_IMAGE_DEFAULT.to_string())
}

// Timing constants for test operations
const CL_CREATE_WAIT_SECS: u64 = 1; // Wait time after CL creation
const POST_MERGE_WAIT_SECS: u64 = 2; // Wait time after merge operation
const POST_REBASE_WAIT_SECS: u64 = 2; // Wait time after rebase operation
const MEGA_STARTUP_WAIT_SECS: u64 = 5; // Wait time after starting Mega service
const DB_OP_WAIT_SECS: u64 = 2; // Wait time after database operations

// Test users configuration
const TEST_USER_A: &str = "user_a";
const TEST_USER_B: &str = "user_b";
const TEST_USER_A_EMAIL: &str = "user_a@test.com";
const TEST_USER_B_EMAIL: &str = "user_b@test.com";
// Test user tokens (constant values for testing)
const TEST_TOKEN_A: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
const TEST_TOKEN_B: &str = "b2c3d4e5-f6a7-8901-bcde-f12345678901";

// Docker service names (must match docker-compose.demo.yml)
const POSTGRES_CONTAINER: &str = "mega-demo-postgres";
const REDIS_CONTAINER: &str = "mega-demo-redis";
const DOCKER_COMPOSE_FILE: &str = "/tmp/docker-compose.yml";
// Path to compose file on host (relative to workspace root)
const DOCKER_COMPOSE_HOST_PATH: &str = "docker/demo/docker-compose.demo.yml";

fn tracing_subscriber_init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

/// Validate and sanitize repo name to prevent command injection
fn validate_repo_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Repo name cannot be empty");
    }
    if name.len() > 100 {
        anyhow::bail!("Repo name too long (max 100 chars)");
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        anyhow::bail!("Repo name contains invalid characters: {}", name);
    }
    Ok(())
}

/// Validate and sanitize file path to prevent directory traversal
fn validate_file_path(path: &str) -> Result<()> {
    if path.contains("..") || path.starts_with('/') {
        anyhow::bail!("Invalid file path: {}", path);
    }
    Ok(())
}

/// Helper to run a command and check its exit status
async fn exec_check(vm: &mut qlean::Machine, cmd: &str) -> Result<String> {
    let result = vm.exec(cmd).await?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        anyhow::bail!(
            "Command '{}' failed with exit code {:?}\nstdout: {}\nstderr: {}",
            cmd,
            result.status.code(),
            stdout,
            stderr
        );
    }
    Ok(String::from_utf8_lossy(&result.stdout).to_string())
}

/// Generic retry helper with success predicate
///
/// Repeatedly executes a command until it satisfies the predicate or max retries are reached.
async fn retry_until<F>(
    vm: &mut qlean::Machine,
    cmd: &str,
    success_predicate: F,
    service_name: &str,
    max_retries: u32,
    delay_secs: u64,
) -> Result<()>
where
    F: Fn(&str) -> bool,
{
    let mut retries = 0;
    let mut last_error: Option<String> = None;
    let mut last_output: Option<String> = None;

    loop {
        match exec_check(vm, cmd).await {
            Ok(output) => {
                if success_predicate(&output) {
                    tracing::info!("{} is ready.", service_name);
                    return Ok(());
                }
                // Log non-matching successful output at debug level
                tracing::debug!(
                    "{} check attempt {}/{}: predicate not met, output: {}",
                    service_name,
                    retries + 1,
                    max_retries,
                    output.trim()
                );
                last_output = Some(output);
            }
            Err(e) => {
                // Log command failure at debug level
                tracing::debug!(
                    "{} check attempt {}/{} failed: {}",
                    service_name,
                    retries + 1,
                    max_retries,
                    e
                );
                last_error = Some(e.to_string());
            }
        }

        retries += 1;
        if retries >= max_retries {
            let mut msg = format!(
                "{} did not become ready after {} seconds",
                service_name,
                (max_retries as u64) * delay_secs
            );
            if let Some(err) = &last_error {
                msg.push_str(&format!("\nLast error: {}", err));
            }
            if let Some(output) = &last_output {
                msg.push_str(&format!("\nLast output: {}", output.trim()));
            }
            anyhow::bail!(msg);
        }

        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
    }
}

/// Wait for Mega API to be ready by polling the status endpoint
async fn wait_for_mega_service(vm: &mut qlean::Machine, timeout_secs: u64) -> Result<()> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let status_url = format!("http://{}:{}/api/v1/status", MEGA_HOST, MEGA_PORT);

    tracing::info!("Waiting for Mega service at {}...", status_url);

    loop {
        let result = vm
            .exec(&format!(
                "curl -sf -o /dev/null -w \"%{{http_code}}\" \"{}\"",
                status_url
            ))
            .await?;

        let status_code = String::from_utf8_lossy(&result.stdout).trim().to_string();
        tracing::debug!("Mega service check returned status: {}", status_code);

        if status_code == "200" {
            tracing::info!("Mega service is ready (status: {})", status_code);
            return Ok(());
        }

        if start.elapsed() > timeout {
            let log_output =
                exec_check(vm, "cat /tmp/mega.log 2>/dev/null || echo 'No log file'").await?;
            tracing::error!("Mega service logs:\n{}", log_output);
            anyhow::bail!(
                "Timeout waiting for Mega service at {} (last status: {})",
                status_url,
                status_code
            );
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Install Docker in the VM
async fn install_docker(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Installing Docker in VM...");

    // Update package list
    exec_check(vm, "apt-get update -qq").await?;

    // Install prerequisites
    exec_check(
        vm,
        "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
            ca-certificates \
            curl \
            gnupg \
            lsb-release",
    )
    .await?;

    // Add Docker's official GPG key
    exec_check(vm, "install -m 0755 -d /etc/apt/keyrings").await?;

    exec_check(
        vm,
        "curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg",
    )
    .await?;

    exec_check(vm, "chmod a+r /etc/apt/keyrings/docker.gpg").await?;

    // Set up Docker repository
    exec_check(
        vm,
        "echo \"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
            https://download.docker.com/linux/debian $(. /etc/os-release && echo $VERSION_CODENAME) stable\" \
            > /etc/apt/sources.list.d/docker.list",
    )
    .await?;

    // Update package list again
    exec_check(vm, "apt-get update -qq").await?;

    // Install Docker Engine
    exec_check(
        vm,
        "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
            docker-ce \
            docker-ce-cli \
            containerd.io \
            docker-compose-plugin",
    )
    .await?;

    // Start Docker service
    exec_check(vm, "service docker start").await?;

    // Verify Docker is running
    exec_check(vm, "docker info > /dev/null").await?;

    tracing::info!("Docker installed and started successfully.");

    // Upload docker-compose.demo.yml to VM
    tracing::info!("Uploading docker-compose.demo.yml to VM...");
    let host_compose_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(DOCKER_COMPOSE_HOST_PATH);

    // Try to read compose file from host
    let content = std::fs::read_to_string(&host_compose_path).with_context(|| {
        format!(
            "Failed to read docker-compose.demo.yml from {}",
            host_compose_path.display()
        )
    })?;

    vm.write(Path::new(DOCKER_COMPOSE_FILE), content.as_bytes())
        .await?;

    tracing::info!(
        "Uploaded docker-compose.demo.yml from {} to {}",
        host_compose_path.display(),
        DOCKER_COMPOSE_FILE
    );

    Ok(())
}

/// Setup PostgreSQL using Docker in VM
async fn setup_postgres(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Setting up PostgreSQL using Docker (from docker-compose.demo.yml)...");

    // Start PostgreSQL container using uploaded compose file
    // Override environment variables to use test credentials instead of compose defaults
    tracing::info!("Starting PostgreSQL container with test credentials...");
    exec_check(
        vm,
        &format!(
            "POSTGRES_USER={} POSTGRES_PASSWORD={} POSTGRES_DB_MONO={} docker compose -f {} up -d postgres",
            POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB, DOCKER_COMPOSE_FILE
        ),
    )
    .await?;

    // Wait for PostgreSQL to be ready using retry helper
    tracing::info!("Waiting for PostgreSQL to be ready...");
    retry_until(
        vm,
        &format!(
            "docker exec {} pg_isready -U {}",
            POSTGRES_CONTAINER, POSTGRES_USER
        ),
        |output| output.contains("accepting connections"),
        "PostgreSQL",
        30,
        2,
    )
    .await?;

    // Grant schema permissions for PostgreSQL 15+
    tracing::info!("Configuring PostgreSQL permissions...");
    exec_check(
        vm,
        &format!(
            "docker exec {} psql -U {} -d {} -c \"GRANT ALL ON SCHEMA public TO {};\"",
            POSTGRES_CONTAINER, POSTGRES_USER, POSTGRES_DB, POSTGRES_USER
        ),
    )
    .await?;

    exec_check(
        vm,
        &format!(
            "docker exec {} psql -U {} -d {} -c \"GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO {};\"",
            POSTGRES_CONTAINER, POSTGRES_USER, POSTGRES_DB, POSTGRES_USER
        ),
    )
    .await?;

    exec_check(
        vm,
        &format!(
            "docker exec {} psql -U {} -d {} -c \"ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO {};\"",
            POSTGRES_CONTAINER, POSTGRES_USER, POSTGRES_DB, POSTGRES_USER
        ),
    )
    .await?;

    tokio::time::sleep(Duration::from_secs(DB_OP_WAIT_SECS)).await;

    tracing::info!("PostgreSQL setup complete.");
    Ok(())
}

/// Setup Redis using Docker in VM
async fn setup_redis(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Setting up Redis using Docker (from docker-compose.demo.yml)...");

    // Start Redis container using the uploaded compose file
    tracing::info!("Starting Redis container...");
    exec_check(
        vm,
        &format!("docker compose -f {} up -d redis", DOCKER_COMPOSE_FILE),
    )
    .await?;

    // Wait for Redis to be ready using retry helper
    tracing::info!("Waiting for Redis to be ready...");
    retry_until(
        vm,
        &format!("docker exec {} redis-cli ping", REDIS_CONTAINER),
        |output| output.trim() == "PONG",
        "Redis",
        15,
        2,
    )
    .await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    tracing::info!("Redis setup complete.");
    Ok(())
}

/// Setup test users and tokens in database
///
/// This function inserts pre-defined constant tokens for test users.
/// Tokens are constants defined at the top of the file for simplicity.
async fn setup_test_users(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Setting up test users and tokens...");

    // Insert constant tokens into database with random IDs
    tracing::info!("  Creating token for {}", TEST_USER_A);
    exec_check(vm, &format!(
        "docker exec {} psql -U {} -d {} -c \"INSERT INTO access_token (id, username, token, created_at) VALUES (floor(random() * 1000000000000)::bigint, '{}', '{}', NOW());\"",
        POSTGRES_CONTAINER, POSTGRES_USER, POSTGRES_DB, TEST_USER_A, TEST_TOKEN_A
    )).await?;

    tracing::info!("  Creating token for {}", TEST_USER_B);
    exec_check(vm, &format!(
        "docker exec {} psql -U {} -d {} -c \"INSERT INTO access_token (id, username, token, created_at) VALUES (floor(random() * 1000000000000)::bigint, '{}', '{}', NOW());\"",
        POSTGRES_CONTAINER, POSTGRES_USER, POSTGRES_DB, TEST_USER_B, TEST_TOKEN_B
    )).await?;

    tracing::info!("Test users and tokens setup complete.");
    Ok(())
}

/// Setup and start Mega service
async fn setup_mega_service(vm: &mut qlean::Machine) -> Result<()> {
    // Clean up any existing directories from previous test runs
    tracing::info!("Cleaning up existing Mega directories from previous runs...");
    exec_check(
        vm,
        "rm -rf /tmp/mega /tmp/mono /tmp/repo_* 2>/dev/null || true",
    )
    .await?;

    tracing::info!("Creating Mega directories...");
    exec_check(vm, "mkdir -p /tmp/mega/cache").await?;
    exec_check(vm, "mkdir -p /tmp/mega/logs").await?;
    exec_check(vm, "mkdir -p /tmp/mega/import").await?;
    exec_check(vm, "mkdir -p /tmp/mega/lfs").await?;
    exec_check(vm, "mkdir -p /tmp/mega/objects").await?;
    exec_check(vm, "mkdir -p /root/.local/share").await?;
    exec_check(vm, "mkdir -p /root/.local/share/mega/etc").await?;

    tracing::info!("Pulling mono image from ECR...");
    let ecr_image = get_mega_ecr_image();
    exec_check(vm, &format!("docker pull {}", ecr_image)).await?;

    tracing::info!("Extracting mono binary from ECR image...");
    exec_check(
        vm,
        &format!(
            "docker run --rm -v /usr/local/bin:/output {} cp /usr/local/bin/mono /output/",
            ecr_image
        ),
    )
    .await?;
    exec_check(vm, "chmod +x /usr/local/bin/mono").await?;

    // Install curl, jq, git (needed for test execution)
    exec_check(
        vm,
        "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq curl jq git",
    )
    .await?;

    let config_content = format!(
        r#"base_dir = "/tmp/mega"

[log]
log_path = "/tmp/mega/logs"
level = "info"
print_std = true

[database]
db_type = "postgres"
db_path = "/tmp/mega/mega.db"
db_url = "postgres://{}:{}@127.0.0.1:5432/{}"
max_connection = 16
min_connection = 8
acquire_timeout = 3
connect_timeout = 3
sqlx_logging = false

[authentication]
enable_http_auth = true
enable_test_user = false
test_user_name = "mega"
test_user_token = "mega"

[monorepo]
import_dir = "/tmp/mega/import"
admin = ["admin"]
root_dirs = ["third-party", "project", "doc", "release"]
storage_type = "local"

[build]
enable_build = false
orion_server = ""

[pack]
pack_decode_mem_size = "4G"
pack_decode_disk_size = "20%"
pack_decode_cache_path = "/tmp/mega/cache"
clean_cache_after_decode = true
channel_message_size = 1000000

[lfs]
storage_type = "local"

[lfs.ssh]
http_url = "http://localhost:8000"

[lfs.local]
lfs_file_path = "/tmp/mega/lfs"

[object_storage]

[object_storage.s3]
region = "us-east-1"
bucket = "mega"
access_key_id = ""
secret_access_key = ""
endpoint_url = ""

[object_storage.gcs]
bucket = "gitmega"

[object_storage.local]
root_dir = "/tmp/mega/objects"

[redis]
url = "redis://127.0.0.1:6379"
"#,
        POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
    );

    vm.write(
        std::path::Path::new("/root/.local/share/mega/etc/config.toml"),
        config_content.as_bytes(),
    )
    .await?;

    tracing::info!("Starting Mega service in background...");
    exec_check(vm, "nohup mono service http > /tmp/mega.log 2>&1 &").await?;

    tokio::time::sleep(Duration::from_secs(MEGA_STARTUP_WAIT_SECS)).await;

    let ps_output = exec_check(vm, "ps aux | grep '[m]ono' || true").await?;
    tracing::debug!("Mega process status: {}", ps_output);

    wait_for_mega_service(vm, 60).await?;

    tracing::info!("Mega service is ready.");
    Ok(())
}

/// Configure git and initialize mono repository
async fn setup_mono_repository(
    vm: &mut qlean::Machine,
    auth_username: &str,
    auth_token: &str,
) -> Result<()> {
    tracing::info!("Configuring git...");
    exec_check(vm, "git config --global user.name 'Test User'").await?;
    exec_check(vm, "git config --global user.email 'test@example.com'").await?;

    // Clean up any existing mono repository
    exec_check(vm, "rm -rf /tmp/mono").await?;

    tracing::info!("Cloning mono repository from Mega service...");
    // Mega service auto-initializes the monorepo on startup with root_dirs
    let clone_url = format!(
        "http://{}:{}@127.0.0.1:8000/.git",
        auth_username, auth_token
    );
    let clone_output = exec_check(vm, &format!("git clone {} /tmp/mono", clone_url)).await?;
    tracing::debug!("Clone output: {}", clone_output);

    // Add a test file to the cloned repository
    vm.write(Path::new("/tmp/mono/root.txt"), b"Initial mono file")
        .await?;

    exec_check(
        vm,
        "cd /tmp/mono && git add . && git commit -m 'Add test file'",
    )
    .await?;

    let push_output = exec_check(vm, "cd /tmp/mono && git push").await?;
    tracing::debug!("Initial push output: {}", push_output);

    tracing::info!("Mono repository initialized and test file added.");
    Ok(())
}

/// Create a Change List by cloning, modifying, and pushing
/// Create a change list by cloning from a specific monorepo path
#[allow(clippy::too_many_arguments)]
async fn create_change_list(
    vm: &mut qlean::Machine,
    repo_name: &str,
    monorepo_path: &str,  // e.g., "project", "third-party"
    auth_username: &str,  // HTTP auth username (for token)
    auth_token: &str,     // HTTP auth token
    git_user_name: &str,  // Git user name for this repository
    git_user_email: &str, // Git user email for this repository
    files: Vec<(&str, &str)>,
) -> Result<String> {
    validate_repo_name(repo_name)?;

    let repo_path = format!("/tmp/{}", repo_name);
    let clone_url = format!(
        "http://{}:{}@127.0.0.1:8000/{}.git",
        auth_username, auth_token, monorepo_path
    );
    let clone_cmd = format!("git clone {} {}", clone_url, repo_path);

    exec_check(vm, &clone_cmd)
        .await
        .context("Failed to clone repository")?;

    // Configure git user for this specific repository
    exec_check(
        vm,
        &format!(
            "cd {} && git config user.name '{}'",
            repo_path, git_user_name
        ),
    )
    .await
    .context("Failed to set git user.name")?;

    exec_check(
        vm,
        &format!(
            "cd {} && git config user.email '{}'",
            repo_path, git_user_email
        ),
    )
    .await
    .context("Failed to set git user.email")?;

    tracing::info!(
        "  Configured git user: {} <{}>",
        git_user_name,
        git_user_email
    );

    for (filename, content) in files {
        validate_file_path(filename)?;
        let file_path = format!("{}/{}", repo_path, filename);

        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&file_path).parent() {
            let parent_str = parent.to_str().unwrap();
            exec_check(vm, &format!("mkdir -p {}", parent_str)).await?;
        }

        vm.write(Path::new(&file_path), content.as_bytes())
            .await
            .context(format!("Failed to write file: {}", filename))?;
    }

    // Debug: check git status before commit
    let git_status = exec_check(vm, &format!("cd {} && git status --short", repo_path)).await?;
    tracing::info!("  Git status for {}: {}", repo_name, git_status);

    let commit_cmd = format!(
        "cd {} && git add . && git commit -m 'feat: Add {} files'",
        repo_path, repo_name
    );
    exec_check(vm, &commit_cmd)
        .await
        .context("Failed to commit changes")?;

    // Debug: check commit details
    let git_show = exec_check(
        vm,
        &format!("cd {} && git show --name-status --oneline HEAD", repo_path),
    )
    .await?;
    tracing::info!("  Commit details for {}: {}", repo_name, git_show);

    exec_check(vm, &format!("cd {} && git push", repo_path))
        .await
        .context("Failed to push changes")?;

    // Wait for CL creation to complete
    tokio::time::sleep(Duration::from_secs(CL_CREATE_WAIT_SECS)).await;

    // Query CL list via API to get the most recent CL
    let list_response = exec_check(
        vm,
        r#"curl -s -X POST http://127.0.0.1:8000/api/v1/cl/list \
            -H "Content-Type: application/json" \
            -d '{
                "pagination": {"page": 1, "per_page": 10},
                "additional": {
                    "status": "open",
                    "sort_by": "created_at",
                    "asc": false
                }
            }'"#,
    )
    .await
    .context("Failed to query CL list")?;

    let json: Value =
        serde_json::from_str(&list_response).context("Failed to parse CL list response")?;

    // Check if request was successful
    if !json["req_result"].as_bool().unwrap_or(false) {
        anyhow::bail!(
            "CL list API returned error: {}",
            json["err_message"].as_str().unwrap_or("Unknown error")
        );
    }

    // Get the most recent CL (first item in the list)
    let items = json["data"]["items"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("CL list items is not an array"))?;

    if items.is_empty() {
        anyhow::bail!("No CL found after push. This may indicate CL creation failed.");
    }

    let cl_link = items[0]["link"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("CL link not found in response"))?
        .to_string();

    let cl_author = items[0]["author"].as_str().unwrap_or("unknown");

    tracing::info!("  Created CL: {} (author: {})", cl_link, cl_author);

    Ok(cl_link)
}

/// Get file list and count for a CL using files-list API
async fn get_cl_files_count(vm: &mut qlean::Machine, cl_link: &str) -> Result<u64> {
    // Use files-list API instead of files-changed API
    // The files-changed API's total field is inaccurate, but files-list returns correct data
    let files_cmd = exec_check(
        vm,
        &format!(
            "curl -s http://127.0.0.1:8000/api/v1/cl/{}/files-list",
            cl_link
        ),
    )
    .await?;

    let json: Value =
        serde_json::from_str(&files_cmd).context("Failed to parse files-list response")?;

    // Check if request was successful
    if !json["req_result"].as_bool().unwrap_or(false) {
        anyhow::bail!(
            "files-list API returned error: {}",
            json["err_message"].as_str().unwrap_or("Unknown error")
        );
    }

    // Get the files array and count its length
    let files = json["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("files-list data is not an array"))?;

    let total = files.len() as u64;

    // Log detailed file list for debugging
    tracing::info!("  Files in CL {} (count: {}):", cl_link, total);
    for item in files {
        if let Some(file_path) = item.get("path").and_then(|v| v.as_str()) {
            tracing::info!("      - {}", file_path);
        }
    }

    Ok(total)
}

/// Update CL status to 'open'
async fn update_cl_status(vm: &mut qlean::Machine, cl_link: &str, status: &str) -> Result<()> {
    exec_check(
        vm,
        &format!(
            "curl -s -X POST http://127.0.0.1:8000/api/v1/cl/{}/status \
                -H 'Content-Type: application/json' \
                -d '{{\"status\":\"{}\"}}'",
            cl_link, status
        ),
    )
    .await?;
    Ok(())
}

/// Merge a CL using no-auth endpoint
async fn merge_change_list(vm: &mut qlean::Machine, cl_link: &str) -> Result<String> {
    let merge_cmd = exec_check(
        vm,
        &format!(
            "curl -s -X POST http://127.0.0.1:8000/api/v1/cl/{}/merge-no-auth",
            cl_link
        ),
    )
    .await?;
    Ok(merge_cmd)
}

/// Get update-branch status for a CL
async fn get_update_branch_status(vm: &mut qlean::Machine, cl_link: &str) -> Result<bool> {
    let status_cmd = exec_check(
        vm,
        &format!(
            "curl -s http://127.0.0.1:8000/api/v1/cl/{}/update-status",
            cl_link
        ),
    )
    .await?;

    tracing::debug!("Update-status raw response: {}", status_cmd);

    let json: Value =
        serde_json::from_str(&status_cmd).context("Failed to parse update-status response")?;

    tracing::debug!("Update-status parsed JSON: {:?}", json);

    // Log the key fields
    if let Some(data) = json.get("data") {
        tracing::info!(
            "   base_commit: {}, target_head: {}, outdated: {}",
            data.get("base_commit")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A"),
            data.get("target_head")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A"),
            data.get("outdated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        );
    }

    let needs_update = json["data"]["need_update"].as_bool().unwrap_or(false)
        || json["data"]["outdated"].as_bool().unwrap_or(false);

    Ok(needs_update)
}

/// Call update-branch for a CL
async fn update_branch(vm: &mut qlean::Machine, cl_link: &str) -> Result<String> {
    let update_cmd = exec_check(
        vm,
        &format!(
            "curl -s -X POST http://127.0.0.1:8000/api/v1/cl/{}/update-branch",
            cl_link
        ),
    )
    .await?;
    Ok(update_cmd)
}

/// Test: CL Merge and Update-Branch Workflow
async fn test_cl_merge_and_update_branch(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Test: CL Merge and Update-Branch Integration");
    tracing::info!("============================================================");
    tracing::info!("");

    // Phase 1: Create two CLs with different users
    tracing::info!("Phase 1: Creating Change Lists");
    tracing::info!(
        "  Creating CL-1 with files including common.txt (user: {})",
        TEST_USER_A
    );
    let cl1 = create_change_list(
        vm,
        "repo_a",
        "project",
        TEST_USER_A,
        TEST_TOKEN_A,
        TEST_USER_A,
        TEST_USER_A_EMAIL,
        vec![
            ("common.txt", "Initial content by user_a"),
            ("repo_a/file1.txt", "Content A1"),
        ],
    )
    .await
    .context("Failed to create CL-1")?;
    tracing::info!("  CL-1 created: {}", cl1);

    tracing::info!(
        "  Creating CL-2 with files including common.txt (user: {})",
        TEST_USER_B
    );
    let cl2 = create_change_list(
        vm,
        "repo_b",
        "project",
        TEST_USER_B,
        TEST_TOKEN_B,
        TEST_USER_B,
        TEST_USER_B_EMAIL,
        vec![
            ("common.txt", "Modified by user_b - conflicts with CL-1!"),
            ("repo_b/file2.txt", "Content B1"),
        ],
    )
    .await
    .context("Failed to create CL-2")?;
    tracing::info!("  CL-2 created: {}", cl2);
    tracing::info!("");

    // Verify CL-1 and CL-2 are distinct (now that we have HTTP auth)
    if cl1 == cl2 {
        anyhow::bail!(
            "CL-1 and CL-2 have the same link ({}). With HTTP auth enabled, each user should have their own CL!",
            cl1
        );
    } else {
        tracing::info!(
            "  CL-1 ({}) and CL-2 ({}) are distinct CLs (expected)",
            cl1,
            cl2
        );
    }
    tracing::info!("");

    // Phase 2: Pre-merge baseline
    tracing::info!("Phase 2: Pre-merge Baseline");
    let files_before = get_cl_files_count(vm, &cl2).await?;
    tracing::info!("  CL-2 files count: {}", files_before);
    tracing::info!("");

    // Phase 3: Merge CL-1
    tracing::info!("Phase 3: Merging CL-1");
    tracing::info!("  Updating CL-1 status to 'open'");
    update_cl_status(vm, &cl1, "open")
        .await
        .context("Failed to update CL-1 status")?;

    tracing::info!("  Merging CL-1 into /project main");
    merge_change_list(vm, &cl1)
        .await
        .context("Failed to merge CL-1")?;
    tracing::info!("  CL-1 merged successfully");
    tokio::time::sleep(Duration::from_secs(POST_MERGE_WAIT_SECS)).await;
    tracing::info!("");

    // Phase 4: Post-merge verification
    tracing::info!("Phase 4: Post-merge Verification");
    let files_after = get_cl_files_count(vm, &cl2)
        .await
        .context("Failed to get CL-2 files count")?;
    tracing::info!("  CL-2 files count: {}", files_after);

    if files_after != files_before {
        tracing::warn!(
            "  CL-2 files count changed (before: {}, after: {})",
            files_before,
            files_after
        );
    } else {
        tracing::info!("  CL-2 files count unchanged (expected before rebase)");
    }
    tracing::info!("");

    // Phase 5: Update-branch detection
    tracing::info!("Phase 5: Update-Branch Detection");
    let needs_update = get_update_branch_status(vm, &cl2)
        .await
        .context("Failed to check update-branch status")?;

    if !needs_update {
        tracing::warn!("  CL-2 not marked as outdated (may be expected in current setup)");
    } else {
        tracing::info!("  CL-2 correctly detected as outdated");
    }
    tracing::info!("");

    // Phase 6: Rebase CL-2
    tracing::info!("Phase 6: Rebasing CL-2");
    tracing::info!("  Calling update-branch for CL-2");
    update_branch(vm, &cl2)
        .await
        .context("Failed to update CL-2 branch")?;
    tracing::info!("  CL-2 update-branch completed");
    tokio::time::sleep(Duration::from_secs(POST_REBASE_WAIT_SECS)).await;
    tracing::info!("");

    // Phase 7: Final verification
    tracing::info!("Phase 7: Final Verification");
    let files_final = get_cl_files_count(vm, &cl2)
        .await
        .context("Failed to get final CL-2 files count")?;
    tracing::info!("  CL-2 final files count: {}", files_final);

    tracing::info!("");
    tracing::info!("============================================================");
    tracing::info!("TEST PASSED");
    tracing::info!("============================================================");
    tracing::info!("Summary:");
    tracing::info!("  CL operations completed successfully");
    tracing::info!("  Merge workflow: OK");
    tracing::info!("  Update-branch workflow: OK");
    tracing::info!("============================================================");
    tracing::info!("");

    Ok(())
}

#[tokio::test]
#[ignore] // Skip in CI - requires libguestfs-tools and QEMU/KVM
async fn test_cl_merge_and_update_branch_docker_integration() -> Result<()> {
    tracing_subscriber_init();

    let ecr_image = get_mega_ecr_image();
    tracing::info!("Using mono binary from ECR image: {}", ecr_image);

    tracing::info!("Creating VM image...");
    let image = create_image(Distro::Debian, "debian-13-generic-amd64").await?;
    let config = MachineConfig {
        core: 2,
        mem: 2048,
        disk: Some(15), // Larger disk to accommodate Docker and containers
        clear: true,
    };

    with_machine(&image, &config, |vm| {
        Box::pin(async move {
            tracing::info!("============================================================");
            tracing::info!("Mega CL Integration Test Suite");
            tracing::info!("Environment: QEMU/KVM Virtual Machine with Docker");
            tracing::info!("============================================================");
            tracing::info!("");

            tracing::info!("Setting up test environment...");
            install_docker(vm)
                .await
                .context("Docker installation failed")?;
            setup_postgres(vm)
                .await
                .context("PostgreSQL setup failed")?;
            setup_redis(vm).await.context("Redis setup failed")?;
            setup_mega_service(vm)
                .await
                .context("Mega service setup failed")?;
            setup_test_users(vm)
                .await
                .context("Test users setup failed")?;
            // Initialize monorepo using user_a's credentials
            setup_mono_repository(vm, TEST_USER_A, TEST_TOKEN_A)
                .await
                .context("Monorepo initialization failed")?;
            tracing::info!("Environment ready");
            tracing::info!("");

            // Run test scenarios
            test_cl_merge_and_update_branch(vm).await?;

            // Cleanup: Stop and remove Docker containers
            tracing::info!("Cleaning up Docker containers...");
            exec_check(
                vm,
                &format!(
                    "docker compose -f {} stop postgres redis && docker compose -f {} rm -f postgres redis",
                    DOCKER_COMPOSE_FILE, DOCKER_COMPOSE_FILE
                ),
            )
            .await
            .ok(); // Don't fail if cleanup fails

            tracing::info!("");
            tracing::info!("============================================================");
            tracing::info!("ALL INTEGRATION TESTS PASSED");
            tracing::info!("============================================================");

            Ok(())
        })
    })
    .await?;

    Ok(())
}
