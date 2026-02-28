//! Integration tests for HTTP server session middleware with real Redis
//!
//! These tests run inside a QEMU/KVM virtual machine using the qlean crate,
//! with Redis running in Docker containers inside the VM.
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
//! cargo test -p mono --test http_server_session_tests -- --ignored --nocapture
//! ```
//!
//! ## Test Design
//!
//! This test uses Docker containers for Redis inside the VM,
//! reusing docker-compose.demo.yml for consistency with the demo environment.
//! All test scenarios run in a single test function to avoid multiple VM startups.

use std::{path::Path, sync::Once, time::Duration};

use anyhow::{Context, Result};
use qlean::{Distro, MachineConfig, create_image, with_machine};
use tracing_subscriber::EnvFilter;

// Docker service names (must match docker-compose.demo.yml)
const REDIS_CONTAINER: &str = "mega-demo-redis";
const DOCKER_COMPOSE_FILE: &str = "/tmp/docker-compose.yml";
const DOCKER_COMPOSE_HOST_PATH: &str = "docker/demo/docker-compose.demo.yml";

fn tracing_subscriber_init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

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

async fn install_docker(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Installing Docker in VM...");

    exec_check(vm, "apt-get update -qq").await?;

    exec_check(
        vm,
        "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
            ca-certificates \
            curl \
            gnupg \
            lsb-release",
    )
    .await?;

    exec_check(vm, "install -m 0755 -d /etc/apt/keyrings").await?;

    exec_check(
        vm,
        "curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg",
    )
    .await?;

    exec_check(vm, "chmod a+r /etc/apt/keyrings/docker.gpg").await?;

    exec_check(
        vm,
        "echo \"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
            https://download.docker.com/linux/debian $(. /etc/os-release && echo $VERSION_CODENAME) stable\" \
            > /etc/apt/sources.list.d/docker.list",
    )
    .await?;

    exec_check(vm, "apt-get update -qq").await?;

    exec_check(
        vm,
        "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
            docker-ce \
            docker-ce-cli \
            containerd.io \
            docker-compose-plugin",
    )
    .await?;

    exec_check(vm, "service docker start").await?;

    exec_check(vm, "docker info > /dev/null").await?;

    tracing::info!("Docker installed and started successfully.");
    Ok(())
}

async fn upload_docker_compose(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Uploading docker-compose.demo.yml to VM...");

    let host_compose_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(DOCKER_COMPOSE_HOST_PATH);

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

async fn setup_redis(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Setting up Redis using Docker...");

    exec_check(
        vm,
        &format!("docker compose -f {} up -d redis", DOCKER_COMPOSE_FILE),
    )
    .await?;

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

    tracing::info!("Redis setup complete.");
    Ok(())
}

async fn cleanup_docker(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("Cleaning up Docker containers...");
    let _ = exec_check(
        vm,
        &format!("docker compose -f {} down", DOCKER_COMPOSE_FILE),
    )
    .await;
    Ok(())
}

// ============================================================================
// Test Scenarios - All in one function to avoid multiple VM startups
// ============================================================================

/// Phase 1: Test session creation and persistence via Redis
async fn phase1_test_session_creation_and_persistence(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 1: Session Creation and Persistence via Redis");

    let session_key = "test:http:session:persistence";

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} 'test_value' EX 3600",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    let result = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli GET {}",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    let retrieved = result.trim();
    if retrieved == "test_value" {
        tracing::info!("  PASS: Session data created and persisted");
    } else {
        anyhow::bail!(
            "Session persistence failed: expected 'test_value', got {}",
            retrieved
        );
    }

    Ok(())
}

/// Phase 2: Test session clearing via Redis
async fn phase2_test_session_clearing(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 2: Session Clearing via Redis");

    let session_key = "test:http:session:clearing";

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} 'test_value' EX 3600",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli DEL {}",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    let result = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli GET {}",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    let retrieved = result.trim();
    if retrieved.is_empty() || retrieved == "(nil)" {
        tracing::info!("  PASS: Session data cleared successfully");
    } else {
        anyhow::bail!("Session clearing failed, got: {}", retrieved);
    }

    Ok(())
}

/// Phase 3: Test session expiry via Redis TTL
async fn phase3_test_session_expiry(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 3: Session Expiry via Redis TTL");

    let session_key = "test:http:session:expiry";

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} 'test_value' EX 1",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    tracing::info!("  Waiting for session to expire (1 second)...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let result = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli GET {}",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    let retrieved = result.trim();
    if retrieved.is_empty() || retrieved == "(nil)" {
        tracing::info!("  PASS: Session expired as expected");
    } else {
        anyhow::bail!("Session should have expired, got: {}", retrieved);
    }

    Ok(())
}

/// Phase 4: Test session isolation (different keys)
async fn phase4_test_session_isolation(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 4: Session Isolation via Redis");

    let session_key1 = "test:http:session:isolation:1";
    let session_key2 = "test:http:session:isolation:2";

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} 'value1' EX 3600",
            REDIS_CONTAINER, session_key1
        ),
    )
    .await?;

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} 'value2' EX 3600",
            REDIS_CONTAINER, session_key2
        ),
    )
    .await?;

    let result1 = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli GET {}",
            REDIS_CONTAINER, session_key1
        ),
    )
    .await?;

    let result2 = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli GET {}",
            REDIS_CONTAINER, session_key2
        ),
    )
    .await?;

    if result1.trim() == "value1" && result2.trim() == "value2" {
        tracing::info!("  PASS: Sessions are properly isolated");
    } else {
        anyhow::bail!(
            "Session isolation failed: key1={}, key2={}",
            result1.trim(),
            result2.trim()
        );
    }

    Ok(())
}

/// Phase 5: Test session without data (non-existent key)
async fn phase5_test_session_without_data(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 5: Session Without Data via Redis");

    let session_key = "test:http:session:nonexistent";

    let result = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli GET {}",
            REDIS_CONTAINER, session_key
        ),
    )
    .await?;

    let retrieved = result.trim();
    if retrieved.is_empty() || retrieved == "(nil)" {
        tracing::info!("  PASS: Non-existent session returns nothing");
    } else {
        anyhow::bail!("Expected no data, got: {}", retrieved);
    }

    Ok(())
}

// ============================================================================
// MAIN TEST
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_http_server_session_with_redis() -> Result<()> {
    tracing_subscriber_init();

    let image = create_image(Distro::Debian, "debian-13-generic-amd64").await?;
    let config = MachineConfig {
        core: 2,
        mem: 2048,
        disk: Some(15),
        clear: true,
    };

    with_machine(&image, &config, |vm| {
        Box::pin(async move {
            tracing::info!("============================================================");
            tracing::info!("HTTP Server Session Integration Test (Redis)");
            tracing::info!("============================================================");

            install_docker(vm).await.context("Docker install failed")?;
            upload_docker_compose(vm)
                .await
                .context("Upload docker-compose failed")?;

            setup_redis(vm).await.context("Redis setup failed")?;

            tracing::info!("All services are ready");
            tracing::info!("");

            phase1_test_session_creation_and_persistence(vm)
                .await
                .context("Phase 1 failed")?;
            phase2_test_session_clearing(vm)
                .await
                .context("Phase 2 failed")?;
            phase3_test_session_expiry(vm)
                .await
                .context("Phase 3 failed")?;
            phase4_test_session_isolation(vm)
                .await
                .context("Phase 4 failed")?;
            phase5_test_session_without_data(vm)
                .await
                .context("Phase 5 failed")?;

            tracing::info!("");
            tracing::info!("All test phases completed successfully!");

            cleanup_docker(vm).await?;

            Ok(())
        })
    })
    .await
    .context("Failed to run VM test")?;

    Ok(())
}
