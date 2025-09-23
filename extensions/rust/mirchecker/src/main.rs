use anyhow::{Result, anyhow};
use clap::Parser;
use dotenvy::dotenv;
use git2::Repository;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::process::Command;
use tracing::{error, info, warn};
use tokio_postgres::{NoTls, Client};

mod model;
use model::{MessageModel, RepoSyncStatus};

// Database handler structure
#[derive(Debug)]
struct DBHandler {
    client: Client,
}

impl DBHandler {
    pub async fn insert_mirchecker_result_into_pg(
        &self,
        id: String,
        result: String,
    ) -> Result<bool> {
        let rows_affected = self.client
            .execute(
                "INSERT INTO mirchecker_res(id, res) VALUES ($1, $2)
                 ON CONFLICT (id) DO UPDATE SET res = $2;",
                &[&id, &result],
            )
            .await
            .map_err(|e| anyhow!("Database operation failed: {}", e))?;
        
        let success = rows_affected > 0;
        if success {
            info!("✅ Successfully saved mir-checker result to database: {}, affected rows: {}", id, rows_affected);
        } else {
            warn!("⚠️ Database operation with no affected rows: {}", id);
        }
        
        Ok(success)
    }
}

// Get database configuration from environment variables
fn db_connection_config_from_env() -> String {
    let host = env::var("PG_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("PG_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = env::var("PG_USER").unwrap_or_else(|_| "postgres".to_string());
    let password = env::var("PG_PASSWORD").unwrap_or_else(|_| "".to_string());
    let dbname = env::var("PG_DB").unwrap_or_else(|_| "postgres".to_string());
    
    format!("host={host} port={port} user={user} password={password} dbname={dbname}")
}

#[derive(Parser, Debug)]
#[command(name = "mir-checker-kafka-consumer")]
#[command(about = "A Kafka consumer that processes git repositories with mir-checker")]
struct Args {
    /// Kafka broker address
    #[arg(long, env = "KAFKA_BROKER", default_value = "172.17.0.1:30092")]
    kafka_broker: String,

    /// Kafka consumer group ID
    #[arg(
        long,
        env = "KAFKA_CONSUMER_GROUP_ID",
        default_value = "mir-checker-consumer-group1"
    )]
    group_id: String,

    /// Kafka topic name
    #[arg(long, env = "KAFKA_TOPIC", default_value = "mir-checker-scan-requests")]
    topic: String,

    /// mir-checker executable path
    #[arg(long, env = "MIR_CHECKER_PATH", default_value = "./cargo-mir-checker")]
    mir_checker_path: String,

    /// Repository clone root directory
    #[arg(long, env = "CLONE_DIR", default_value = "/tmp/mir-checker-repos")]
    clone_dir: String,

    /// Use temporary directory (delete after each analysis)
    #[arg(long, env = "USE_TEMP_DIR", default_value = "true")]
    use_temp_dir: bool,

    /// Verbose log output
    #[arg(short, long, env = "VERBOSE")]
    verbose: bool,
}

/// Consumer instance
struct MirCheckerConsumer {
    consumer: StreamConsumer,
    args: Args,
}

impl MirCheckerConsumer {
    /// Create new consumer instance
    async fn new(args: Args) -> Result<Self> {
        info!("Initializing Kafka consumer...");
        info!("Broker: {}", args.kafka_broker);
        info!("Group ID: {}", args.group_id);
        info!("Topic: {}", args.topic);

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &args.group_id)
            .set("bootstrap.servers", &args.kafka_broker)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "1000")
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.offset.store", "true")
            .create()?;

        consumer.subscribe(&[&args.topic])?;

        info!("Kafka consumer initialization completed");

        Ok(Self { consumer, args })
    }

    /// Start consuming messages
    async fn start_consuming(&self) -> Result<()> {
        info!("🚀 Starting to listen for Kafka messages...");

        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        if let Err(e) = self.process_message(payload).await {
                            error!("Failed to process message: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to receive message: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Process single message
    async fn process_message(&self, payload: &[u8]) -> Result<()> {
        let message_str = std::str::from_utf8(payload)?;
        info!("📨 Received raw message: {}", message_str);

        // Parse as MessageModel directly
        let message_model: MessageModel =
            serde_json::from_str(message_str).map_err(|e| anyhow!("Unable to parse message format: {}", e))?;

        info!("📨 Successfully parsed MessageModel: {:?}", message_model);

        // Process message
        self.process_message_model(&message_model).await?;

        info!("✅ Message processing completed");
        Ok(())
    }

    /// Process MessageModel format message
    async fn process_message_model(&self, message_model: &MessageModel) -> Result<()> {
        info!("🔍 Processing MessageModel message");
        info!("  - Crate name: {}", message_model.db_model.crate_name);
        info!("  - Status: {:?}", message_model.db_model.status);
        info!("  - Message type: {:?}", message_model.message_kind);
        info!("  - Data source: {:?}", message_model.source_of_data);
        info!("  - Timestamp: {}", message_model.timestamp);

        // Check if analysis is needed
        match message_model.db_model.status {
            RepoSyncStatus::Succeed => {
                info!("📥 Repository sync succeeded, starting mir-checker analysis");

                // Get repository URL
                let repo_url = message_model
                    .db_model
                    .github_url
                    .as_ref()
                    .unwrap_or(&message_model.db_model.mega_url);

                // Execute analysis
                self.analyze_repository(message_model, repo_url).await?;
            }
            RepoSyncStatus::Failed => {
                warn!(
                    "⚠️ Repository sync failed, skipping analysis: {:?}",
                    message_model.db_model.err_message
                );
            }
            RepoSyncStatus::Analysing => {
                info!("🔄 Repository is being analyzed, skipping duplicate analysis");
            }
            RepoSyncStatus::Analysed => {
                info!("✅ Repository analysis already completed, skipping");
            }
            RepoSyncStatus::Syncing => {
                info!("🔄 Repository is syncing, waiting for completion");
            }
        }

        Ok(())
    }

    /// Analyze repository
    async fn analyze_repository(&self, message_model: &MessageModel, repo_url: &str) -> Result<()> {
        // Clone repository
        let repo_path = self
            .clone_repository(repo_url, &message_model.db_model.crate_name)
            .await?;

        // Execute mir-checker analysis
        self.run_mir_checker_analysis(message_model, &repo_path).await?;

        Ok(())
    }

    /// Clone Git repository
    async fn clone_repository(&self, repo_url: &str, crate_name: &str) -> Result<PathBuf> {
        info!("📥 Starting to clone repository: {}", repo_url);

        let repo_path = if self.args.use_temp_dir {
            let temp_dir = TempDir::new()?;
            temp_dir.path().join(crate_name)
        } else {
            PathBuf::from(&self.args.clone_dir).join(crate_name)
        };

        if repo_path.exists() {
            tokio::fs::remove_dir_all(&repo_path).await?;
        }

        if let Some(parent) = repo_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let _repo = Repository::clone(repo_url, &repo_path)?;
        info!("✅ Repository clone completed: {:?}", repo_path);

        Ok(repo_path)
    }

    /// Execute mir-checker analysis
    async fn run_mir_checker_analysis(&self, message_model: &MessageModel, repo_path: &PathBuf) -> Result<()> {
        let start_time = std::time::Instant::now();
        let crate_name = &message_model.db_model.crate_name;
        
        info!("🔍 Starting mir-checker analysis: {} -> {:?}", crate_name, repo_path);

        // Create database connection
        let db_connection_config = db_connection_config_from_env();
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
            .await
            .map_err(|e| anyhow!("Database connection failed: {}", e))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Database connection error: {}", e);
            }
        });

        let dbhandler = DBHandler { client };
        
        let id = format!("{}/{}", 
            message_model.db_model.mega_url.replace("http://", "").replace("/", "_"),
            crate_name
        );

        // Step 1: Execute cargo clean
        info!("🧹 Executing cargo clean...");
        let clean_output = Command::new("cargo")
            .arg("clean")
            .current_dir(repo_path)
            .output()
            .await?;

        if !clean_output.status.success() {
            let error_msg = String::from_utf8_lossy(&clean_output.stderr);
            error!("cargo clean failed: {}", error_msg);
            return Err(anyhow!("cargo clean failed: {}", error_msg));
        }
        info!("✅ cargo clean completed");

        // Convert to absolute path
        let mir_checker_absolute_path = if PathBuf::from(&self.args.mir_checker_path).is_absolute() {
            self.args.mir_checker_path.clone()
        } else {
            let current_dir = std::env::current_dir()?;
            current_dir.join(&self.args.mir_checker_path).to_string_lossy().to_string()
        };

        // Step 2: Get entry points
        info!("🔍 Getting entry points...");
        let show_entries_output = Command::new(&mir_checker_absolute_path)
            .arg("mir-checker")
            .arg("--")
            .arg("--show_entries")
            .current_dir(repo_path)
            .output()
            .await?;

        if !show_entries_output.status.success() {
            let error_msg = String::from_utf8_lossy(&show_entries_output.stderr);
            error!("Failed to get entry points: {}", error_msg);
            return Err(anyhow!("Failed to get entry points: {}", error_msg));
        }

        let stdout_str = String::from_utf8(show_entries_output.stdout)?;
        let entries: Vec<String> = stdout_str
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect();

        info!("✅ Found {} entry points", entries.len());
        for entry in &entries {
            info!("  - {}", entry);
        }

        // Step 3: Analyze each entry point
        let mut all_outputs = Vec::new();
        
        for (i, entry) in entries.iter().enumerate() {
            info!("🔧 Analyzing entry point {}/{}: {}", i + 1, entries.len(), entry);

            // Clean before each analysis
            let clean_output = Command::new("cargo")
                .arg("clean")
                .current_dir(repo_path)
                .output()
                .await?;

            if !clean_output.status.success() {
                let error_msg = String::from_utf8_lossy(&clean_output.stderr);
                warn!("Entry point {} cargo clean failed: {}", entry, error_msg);
                continue;
            }

            // Analyze specific entry point
            let analysis_output = Command::new(&mir_checker_absolute_path)
                .arg("mir-checker")
                .arg("--")
                .arg("--entry")
                .arg(entry)
                .current_dir(repo_path)
                .output()
                .await?;

            if !analysis_output.status.success() {
                let error_msg = String::from_utf8_lossy(&analysis_output.stderr);
                warn!("Entry point {} analysis failed: {}", entry, error_msg);
            }

            // Parse warning information
            let warnings = self.parse_mir_checker_warnings(&analysis_output.stderr);
            
            if !warnings.is_empty() {
                info!("⚠️ Entry point {} found {} warnings", entry, warnings.len());
                let combined_warnings = warnings.join("\n");
                all_outputs.push(combined_warnings);
            } else {
                info!("✅ Entry point {} has no warnings", entry);
            }
        }

        let duration = start_time.elapsed();
        let real_res = all_outputs.join("\n");

        // Step 4: Save results to database
        if !real_res.is_empty() {
            info!("📊 mir-checker analysis completed: {} (duration: {:?})", crate_name, duration);
            info!("📋 Found {} warning blocks", all_outputs.len());
            info!("📄 Complete result: {}", real_res);
            
            // Insert results to database
            dbhandler.insert_mirchecker_result_into_pg(id, real_res).await?;
        } else {
            info!("✅ mir-checker analysis completed: {} (duration: {:?}) - no warnings", crate_name, duration);
            // Record to database even if no warnings, indicating analysis completed
            dbhandler.insert_mirchecker_result_into_pg(id, "No warnings found".to_string()).await?;
        }

        Ok(())
    }

    /// Parse mir-checker warning information
    fn parse_mir_checker_warnings(&self, stderr: &[u8]) -> Vec<String> {
        let stderr_str = String::from_utf8_lossy(stderr);
        let mut warning_blocks = Vec::new();
        let mut current_block = String::new();
        let mut in_warning_block = false;

        for line in stderr_str.lines() {
            if line.starts_with("warning: [MirChecker]") {
                // Save collected block (if any)
                if in_warning_block && !current_block.is_empty() {
                    warning_blocks.push(current_block.clone());
                }
                // Start new block
                in_warning_block = true;
                current_block.clear();
                current_block.push_str(line);
                current_block.push('\n');
            } else if in_warning_block {
                if line.starts_with(" INFO") {
                    if !current_block.is_empty() {
                        warning_blocks.push(current_block.clone());
                    }
                    current_block.clear();
                    in_warning_block = false;
                } else {
                    current_block.push_str(line);
                    current_block.push('\n');
                }
            }
        }

        if in_warning_block && !current_block.is_empty() {
            warning_blocks.push(current_block);
        }

        if !warning_blocks.is_empty() {
            info!("🔍 Extracted {} warning blocks in total", warning_blocks.len());
            for (i, block) in warning_blocks.iter().enumerate() {
                if self.args.verbose {
                    info!("Warning block {}:\n{}", i + 1, block);
                }
            }
        }

        warning_blocks
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    if let Err(e) = dotenv() {
        warn!("Unable to load .env file: {}, will use default values or command line arguments", e);
    } else {
        info!("✅ Successfully loaded .env file");
    }

    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(if args.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    info!("🚀 Starting MirChecker Kafka Consumer");
    info!("Configuration: {:?}", args);

    // Check mir-checker availability
    check_mir_checker_available(&args.mir_checker_path).await?;

    // Create and start consumer
    let consumer = MirCheckerConsumer::new(args).await?;
    consumer.start_consuming().await?;

    Ok(())
}

/// Check mir-checker command availability
async fn check_mir_checker_available(mir_checker_path: &str) -> Result<()> {
    info!("🔍 Checking mir-checker availability...");

    let output = Command::new(mir_checker_path).arg("--help").output().await;

    match output {
        Ok(output) if output.status.success() => {
            info!("✅ mir-checker is available");
            Ok(())
        }
        Ok(_) => Err(anyhow!("mir-checker command execution failed")),
        Err(e) => Err(anyhow!(
            "mir-checker command not found: {}. Please ensure mir-checker is installed and in the specified path",
            e
        )),
    }
}