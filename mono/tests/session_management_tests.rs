//! Integration tests for session management with real Redis
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
//! cargo test -p mono --test session_management_tests -- --ignored --nocapture
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

/// Phase 1: Test session creation and retrieval via Redis
async fn phase1_test_session_creation_and_retrieval(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 1: Session Creation and Retrieval via Redis");

    let session_key = "test:session:creation";
    let test_data =
        r#"{"id":1,"username":"testuser","email":"test@example.com","name":"Test User"}"#;

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} '{}' EX 3600",
            REDIS_CONTAINER, session_key, test_data
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
    if retrieved == test_data {
        tracing::info!("  PASS: Session data successfully created and retrieved");
    } else {
        anyhow::bail!(
            "Session data mismatch: expected {}, got {}",
            test_data,
            retrieved
        );
    }

    Ok(())
}

/// Phase 2: Test session clearing via Redis
async fn phase2_test_session_clearing(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 2: Session Clearing via Redis");

    let session_key = "test:session:clearing";
    let test_data = r#"{"id":1,"username":"testuser","email":"test@example.com"}"#;

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} '{}' EX 3600",
            REDIS_CONTAINER, session_key, test_data
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
        tracing::info!("  PASS: Session data successfully cleared");
    } else {
        anyhow::bail!("Session should be cleared but got: {}", retrieved);
    }

    Ok(())
}

/// Phase 3: Test session persistence via Redis
async fn phase3_test_session_persistence(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 3: Session Persistence via Redis");

    let session_key = "test:session:persistence";
    let test_data = r#"{"id":1,"username":"testuser","email":"test@example.com"}"#;

    exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli SET {} '{}' EX 3600",
            REDIS_CONTAINER, session_key, test_data
        ),
    )
    .await?;

    for i in 0..3 {
        let result = exec_check(
            vm,
            &format!(
                "docker exec {} redis-cli GET {}",
                REDIS_CONTAINER, session_key
            ),
        )
        .await?;

        let retrieved = result.trim();
        if retrieved == test_data {
            tracing::info!("  Iteration {}: Session data persisted correctly", i + 1);
        } else {
            anyhow::bail!(
                "Session data mismatch at iteration {}: expected {}, got {}",
                i + 1,
                test_data,
                retrieved
            );
        }
    }

    tracing::info!("  PASS: Session data persisted across multiple retrievals");
    Ok(())
}

/// Phase 4: Test session ID generation (using Redis INCR)
async fn phase4_test_session_id_generation(vm: &mut qlean::Machine) -> Result<()> {
    tracing::info!("============================================================");
    tracing::info!("Phase 4: Session ID Generation via Redis");

    let counter_key = "test:session:counter";

    let result1 = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli INCR {}",
            REDIS_CONTAINER, counter_key
        ),
    )
    .await?;
    let id1 = result1.trim().parse::<i64>()?;

    let result2 = exec_check(
        vm,
        &format!(
            "docker exec {} redis-cli INCR {}",
            REDIS_CONTAINER, counter_key
        ),
    )
    .await?;
    let id2 = result2.trim().parse::<i64>()?;

    if id2 > id1 {
        tracing::info!("  PASS: Generated unique session IDs: {}, {}", id1, id2);
    } else {
        anyhow::bail!("Session IDs should be unique and incrementing");
    }

    Ok(())
}

// ============================================================================
// MAIN TEST
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_session_management_with_redis() -> Result<()> {
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
            tracing::info!("Session Management Integration Test (Redis)");
            tracing::info!("============================================================");

            install_docker(vm).await.context("Docker install failed")?;
            upload_docker_compose(vm)
                .await
                .context("Upload docker-compose failed")?;

            setup_redis(vm).await.context("Redis setup failed")?;

            tracing::info!("All services are ready");
            tracing::info!("");

            phase1_test_session_creation_and_retrieval(vm)
                .await
                .context("Phase 1 failed")?;
            phase2_test_session_clearing(vm)
                .await
                .context("Phase 2 failed")?;
            phase3_test_session_persistence(vm)
                .await
                .context("Phase 3 failed")?;
            phase4_test_session_id_generation(vm)
                .await
                .context("Phase 4 failed")?;

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
