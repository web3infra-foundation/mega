use anyhow::{Result, anyhow};
use clap::Parser;
use git2::Repository;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::process::Command;
use tokio::io::AsyncReadExt;
use tracing::{error, info, warn};
use tokio_postgres::{NoTls, Client};

mod model;
use model::{MessageModel, RepoSyncStatus};

// 添加数据库处理结构
#[derive(Debug)]
struct DBHandler {
    client: Client,
}

impl DBHandler {
    pub async fn insert_sensleak_result_into_pg(
        &self,
        id: String,
        result: String,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let rows_affected = self.client
            .execute(
                "INSERT INTO senseleak_res(id, res) VALUES ($1, $2)
                 ON CONFLICT (id) DO UPDATE SET res = $2;",
                &[&id, &result],
            )
            .await?;
        
        let success = rows_affected > 0;
        if success {
            info!("Successfully upserted record for id: {}, rows affected: {}", id, rows_affected);
        } else {
            warn!("No rows affected for id: {}", id);
        }
        
        Ok(success)
    }
}

// 从环境变量获取数据库配置
fn db_connection_config_from_env() -> String {
    let host = env::var("PG_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("PG_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = env::var("PG_USER").unwrap_or_else(|_| "postgres".to_string());
    let password = env::var("PG_PASSWORD").unwrap_or_else(|_| "".to_string());
    let dbname = env::var("PG_DB").unwrap_or_else(|_| "".to_string());
    format!("host={host} port={port} user={user} password={password} dbname={dbname}")
}

// 提取namespace的辅助函数
async fn extract_namespace(url_str: &str) -> Result<String> {
    info!("enter extract_namespace");
    
    fn remove_dot_git_suffix(input: &str) -> String {
        let input = if input.ends_with('/') {
            input.strip_suffix('/').unwrap()
        } else {
            input
        };

        let input = if input.ends_with(".git") {
            input.strip_suffix(".git").unwrap().to_string()
        } else {
            input.to_string()
        };
        input
    }

    let url = remove_dot_git_suffix(url_str);
    info!("finish get url:{:?}", url);

    let segments: Vec<&str> = url.split("/").collect();
    info!("finish get segments");

    if segments.len() < 2 {
        return Err(anyhow!(
            "URL {} does not include a namespace and a repository name",
            url_str
        ));
    }

    let namespace = format!(
        "{}/{}",
        segments[segments.len() - 2],
        segments[segments.len() - 1]
    );

    Ok(namespace)
}

#[derive(Parser, Debug)]
#[command(name = "analysis-kafka-consumer")]
#[command(about = "A Kafka consumer that processes git repositories with configurable analyzers")]
struct Args {
    /// Kafka broker address
    #[arg(long, env = "KAFKA_BROKER", default_value = "localhost:9092")]
    kafka_broker: String,

    /// Kafka consumer group ID
    #[arg(
        long,
        env = "KAFKA_CONSUMER_GROUP_ID",
        default_value = "analysis-consumer-group1"
    )]
    group_id: String,

    /// Kafka topic name
    #[arg(long, env = "KAFKA_TOPIC", default_value = "analysis-scan-requests")]
    topic: String,

    /// Analyzer configuration file path
    #[arg(long, env = "ANALYZERS_CONFIG", default_value = "analyzers.json")]
    analyzers_config: String,

    /// Repository clone root directory
    #[arg(long, env = "CLONE_DIR", default_value = "/tmp/analysis-repos")]
    clone_dir: String,

    /// Whether to use a temporary directory (deleted after each analysis)
    #[arg(long, env = "USE_TEMP_DIR", default_value = "true")]
    use_temp_dir: bool,

    /// Verbose logging
    #[arg(short, long, env = "VERBOSE")]
    verbose: bool,
}

/// Analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalyzerConfig {
    pub name: String,
    pub executable: String,
    pub args: Vec<String>,
    pub config_file: Option<String>,
    pub working_dir: Option<String>,
    pub env_vars: Option<HashMap<String, String>>,
    pub timeout: Option<u64>,
    pub enabled: bool,
}

/// Analyzer manager
#[derive(Debug)]
struct AnalyzerManager {
    analyzers: HashMap<String, AnalyzerConfig>,
}

impl AnalyzerManager {
    async fn from_config(config_path: &str) -> Result<Self> {
        let config_content = tokio::fs::read_to_string(config_path).await
            .map_err(|e| anyhow!("Failed to read analyzer config file {}: {}", config_path, e))?;
        
        let analyzers_list: Vec<AnalyzerConfig> = serde_json::from_str(&config_content)
            .map_err(|e| anyhow!("Failed to parse analyzer config file: {}", e))?;
        
        let mut analyzers = HashMap::new();
        for analyzer in analyzers_list {
            if analyzer.enabled {
                analyzers.insert(analyzer.name.clone(), analyzer);
            }
        }
        
        info!("Loaded {} analyzers", analyzers.len());
        for analyzer_name in analyzers.keys() {
            info!("  - {}", analyzer_name);
        }
        
        Ok(Self { analyzers })
    }
    
    fn get_enabled_analyzers(&self) -> Vec<&AnalyzerConfig> {
        self.analyzers.values().collect()
    }
}

#[derive(Debug)]
struct AnalysisResult {
    pub analyzer_name: String,
    pub success: bool,
    pub duration: Duration,
}

/// Consumer instance
struct AnalysisConsumer {
    consumer: StreamConsumer,
    analyzer_manager: AnalyzerManager,
    args: Args,
}

impl AnalysisConsumer {
    async fn new(args: Args) -> Result<Self> {
        info!("Initializing Kafka consumer...");
        info!("Broker: {}", args.kafka_broker);
        info!("Group ID: {}", args.group_id);
        info!("Topic: {}", args.topic);

        let analyzer_manager = AnalyzerManager::from_config(&args.analyzers_config).await?;

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &args.group_id)
            .set("bootstrap.servers", &args.kafka_broker)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&[&args.topic])?;

        info!("Kafka consumer initialized");

        Ok(Self { consumer, analyzer_manager, args })
    }

    async fn start_consuming(&self) -> Result<()> {
        info!("Start consuming Kafka messages...");

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

    async fn process_message(&self, payload: &[u8]) -> Result<()> {
        let message_str = std::str::from_utf8(payload)?;
        info!("Received raw message: {}", message_str);

        let message_model: MessageModel =
            serde_json::from_str(message_str).map_err(|e| anyhow!("Failed to parse message format: {}", e))?;

        info!("Parsed MessageModel: {:?}", message_model);

        self.process_message_model(&message_model).await?;

        info!("Message processed successfully");
        Ok(())
    }

    async fn process_message_model(&self, message_model: &MessageModel) -> Result<()> {
        info!("Processing MessageModel message");
        info!("  - Crate name: {}", message_model.db_model.crate_name);
        info!("  - Status: {:?}", message_model.db_model.status);
        info!("  - Message type: {:?}", message_model.message_kind);
        info!("  - Source of data: {:?}", message_model.source_of_data);
        info!("  - Timestamp: {}", message_model.timestamp);

        match message_model.db_model.status {
            RepoSyncStatus::Succeed => {
                info!("Repository sync succeeded, starting analysis");

                let repo_url = message_model
                    .db_model
                    .github_url
                    .as_ref()
                    .unwrap_or(&message_model.db_model.mega_url);

                self.analyze_repository(message_model, repo_url).await?;
            }
            RepoSyncStatus::Failed => {
                warn!(
                    "Repository sync failed, skipping analysis: {:?}",
                    message_model.db_model.err_message
                );
            }
            RepoSyncStatus::Analysing => {
                info!("Repository is currently under analysis, skipping");
            }
            RepoSyncStatus::Analysed => {
                info!("Repository already analyzed, skipping");
            }
            RepoSyncStatus::Syncing => {
                info!("Repository is syncing, waiting for completion");
            }
        }

        Ok(())
    }

    async fn analyze_repository(&self, message_model: &MessageModel, repo_url: &str) -> Result<()> {
        // 从mega_url提取namespace
        let namespace = extract_namespace(&message_model.db_model.mega_url).await?;
        info!("analyze namespace:{}", namespace);

        let repo_path = self
            .clone_repository(repo_url, &message_model.db_model.crate_name)
            .await?;

        let analyzers = self.analyzer_manager.get_enabled_analyzers();
        let mut results = Vec::new();

        for analyzer in analyzers {
            info!("Running analyzer: {}", analyzer.name);
            
            match self.run_analyzer(analyzer, &message_model.db_model.crate_name, &repo_path, &namespace).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    error!("Analyzer {} failed: {}", analyzer.name, e);
                    results.push(AnalysisResult {
                        analyzer_name: analyzer.name.clone(),
                        success: false,
                        duration: Duration::from_secs(0),
                    });
                }
            }
        }

        self.summarize_results(&message_model.db_model.crate_name, &results);

        Ok(())
    }

    async fn clone_repository(&self, repo_url: &str, crate_name: &str) -> Result<PathBuf> {
        info!("Cloning repository: {}", repo_url);

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

        info!("Repo URL: {repo_url:?}, Repo Path: {repo_path:?}");
        let _repo = Repository::clone(repo_url, &repo_path)?;
        info!("Repository cloned: {:?}", repo_path);

        Ok(repo_path)
    }

    async fn run_analyzer(&self, analyzer: &AnalyzerConfig, crate_name: &str, repo_path: &PathBuf, namespace: &str) -> Result<AnalysisResult> {
        let start_time = std::time::Instant::now();
        
        info!("Start analysis with {}: {} -> {:?}", analyzer.name, crate_name, repo_path);

        // 创建输出文件路径
        let output_file = PathBuf::from("/tmp/analysis-output")
            .join(&analyzer.name)
            .join(namespace)
            .join(format!("{crate_name}.txt"));
        
        let output_dir = output_file.parent().unwrap();
        if !output_dir.exists() {
            tokio::fs::create_dir_all(output_dir).await?;
        }

        info!("output_file_path:{:?}", output_file);
        info!("output_dir:{:?}", output_dir);

        let mut cmd = Command::new(&analyzer.executable);

        for arg in &analyzer.args {
            let processed_arg = self.process_argument(arg, repo_path, analyzer, &output_file)?;
            if !processed_arg.is_empty() {
                cmd.arg(processed_arg);
            }
        }

        if let Some(working_dir) = &analyzer.working_dir {
            cmd.current_dir(working_dir);
        }

        if let Some(env_vars) = &analyzer.env_vars {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        if self.args.verbose {
            cmd.arg("--verbose");
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        info!("Executing command: {:?}", cmd);

        let output = if let Some(timeout) = analyzer.timeout {
            tokio::time::timeout(Duration::from_secs(timeout), cmd.output()).await??
        } else {
            cmd.output().await?
        };

        let duration = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        info!("{} stdout:", analyzer.name);
        for line in stdout.lines() {
            info!("  {}", line);
        }

        if !stderr.is_empty() {
            warn!("{} stderr:", analyzer.name);
            for line in stderr.lines() {
                warn!("  {}", line);
            }
        }

        let success = output.status.success();
        if success {
            info!("{} finished successfully: {} (elapsed: {:?})", analyzer.name, crate_name, duration);
            
            // 读取输出文件并保存到数据库
            if let Err(e) = self.save_result_to_database(namespace, &output_file).await {
                error!("Failed to save result to database: {}", e);
            } else {
                info!("Result saved to database for namespace: {}", namespace);
            }
        } else {
            error!("{} failed, exit code: {:?}", analyzer.name, output.status.code());
        }

        Ok(AnalysisResult {
            analyzer_name: analyzer.name.clone(),
            success,
            duration,
        })
    }

    fn process_argument(&self, arg: &str, repo_path: &Path, analyzer: &AnalyzerConfig, output_file: &Path) -> Result<String> {
        let mut result = arg.to_string();
        
        result = result.replace("{repo_path}", &repo_path.to_string_lossy());
        result = result.replace("{repo_name}", &repo_path.file_name().unwrap_or_default().to_string_lossy());
        result = result.replace("{output_file}", &output_file.to_string_lossy());
        
        if let Some(config_file) = &analyzer.config_file {
            result = result.replace("{config_file}", config_file);
        }
        
        Ok(result)
    }

    async fn save_result_to_database(&self, namespace: &str, output_file: &PathBuf) -> Result<()> {
        info!("Saving result to database for namespace: {}", namespace);
        
        // 读取输出文件内容
        let mut file = tokio::fs::File::open(output_file).await?;
        let mut content = String::new();
        file.read_to_string(&mut content).await?;
        
        info!("content:{}", content);
        
        let db_connection_config = db_connection_config_from_env();
        let (client, connection) = tokio_postgres::connect(&db_connection_config, NoTls)
            .await
            .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Database connection error: {}", e);
            }
        });

        let dbhandler = DBHandler { client };
        
        dbhandler
            .insert_sensleak_result_into_pg(namespace.to_string(), content)
            .await
            .map_err(|e| anyhow!("Failed to insert result into database: {}", e))?;

        info!("Result saved to database successfully");
        Ok(())
    }

    fn summarize_results(&self, crate_name: &str, results: &[AnalysisResult]) {
        let total_analyzers = results.len();
        let successful = results.iter().filter(|r| r.success).count();
        let failed = total_analyzers - successful;
        let total_duration: Duration = results.iter().map(|r| r.duration).sum();

        info!("Summary for {}:", crate_name);
        info!("  - Total analyzers: {}", total_analyzers);
        info!("  - Successful: {}", successful);
        info!("  - Failed: {}", failed);
        info!("  - Total elapsed: {:?}", total_duration);

        for result in results {
            let status = if result.success { "Success" } else { "Failed" };
            info!("  {} {}: {:?}", status, result.analyzer_name, result.duration);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(if args.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    info!("Starting analysis Kafka consumer");
    info!("Config: {:?}", args);

    let consumer = AnalysisConsumer::new(args).await?;
    consumer.start_consuming().await?;

    Ok(())
}