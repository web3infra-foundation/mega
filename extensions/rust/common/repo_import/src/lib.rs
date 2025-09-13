mod crate_info;
mod git;
mod kafka_handler;
mod utils;
mod version_info;

extern crate lazy_static;
extern crate pretty_env_logger;

use crate::crate_info::extract_info_local;
use crate::kafka_handler::KafkaHandler;
use crate::utils::{
    extract_namespace, get_program_by_name, insert_namespace_by_repo_path, name_join_version,
    write_into_csv,extract_path_from_segment,
};

//use git::hard_reset_to_head;
use git2::{ObjectType, Oid, Repository};
use model::{repo_sync_model, tugraph_model::*};
use rdkafka::error::KafkaError;
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use url::Url;
use version_info::VersionUpdater;

const CLONE_CRATES_DIR: &str = "/mnt/crates/local_crates_file/";
// const TUGRAPH_IMPORT_FILES_PG: &str = "./tugraph_import_files_mq/";

pub use kafka_handler::reset_kafka_offset;

pub enum MessageKind {
    Mega,
    UserUpload,
}

pub struct ImportMessage<'a> {
    kind: MessageKind,
    message: BorrowedMessage<'a>,
}

pub struct ImportDriver {
    pub context: ImportContext,
    pub import_handler: KafkaHandler,
    pub user_import_handler: KafkaHandler,
    pub sender_handler: KafkaHandler,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Licenses {
    pub program_id: String,
    pub program_name: String,
    pub program_namespace: Option<String>,
    pub license: Option<String>,
}

impl ImportDriver {
    pub async fn new(dont_clone: bool) -> Self {
        tracing::info!("Start to setup Kafka client.");

        let should_reset_kafka_offset = env::var("SHOULD_RESET_KAFKA_OFFSET").unwrap().eq("1");

        let (import_handler, user_import_handler, sender_handler) = init_kafka_handler()
            .await
            .expect("Failed to initialize Kafka handlers");

        let context = if !should_reset_kafka_offset {
            // 如果不需要重置offset，则从checkpoint中恢复context
            let checkpoint_dir =
                env::var("CHECKPOINT_DIR").unwrap_or_else(|_| "./checkpoints".to_string());
            let checkpoint_path = format!("{checkpoint_dir}/latest.json");

            match ImportContext::load_from_file(&checkpoint_path).await {
                Ok(mut ctx) => {
                    // 如果有保存的 offset 且不需要重置到0，则恢复到该位置
                    if let Some(offset) = ctx.kafka_offset {
                        tracing::info!("Restoring Kafka consumer to offset: {}", offset);
                        if let Err(e) = import_handler.seek_to_offset(offset).await {
                            tracing::error!("Failed to seek to offset {}: {}", offset, e);
                        }
                    }
                    ctx.write_tugraph_import_files().await;
                    tracing::info!("Restored context from checkpoint");
                    ctx
                }
                Err(e) => {
                    tracing::warn!("Failed to load checkpoint: {}", e);
                    ImportContext {
                        dont_clone,
                        ..Default::default()
                    }
                }
            }
        } else {
            // 如果需要重置offset，则创建一个新的context
            tracing::info!("Resetting Kafka offset, creating new context");
            ImportContext {
                dont_clone,
                ..Default::default()
            }
        };

        tracing::info!("Finish to setup Kafka client.");

        Self {
            context,
            import_handler,
            user_import_handler,
            sender_handler,
        }
    }

    async fn consume_message(&'_ self) -> Result<ImportMessage<'_>, KafkaError> {
        // try to get data from user_import_handler
        if let Ok(message) = self.user_import_handler.consume_once().await {
            tracing::info!("Receive a user upload message!");
            return Ok(ImportMessage {
                kind: MessageKind::UserUpload,
                message,
            });
        }
        // if there is no data from user_import_handler，try to fetch data from import_handler
        else if let Ok(message) = self.import_handler.consume_once().await {
            return Ok(ImportMessage {
                kind: MessageKind::Mega,
                message,
            });
        };

        Err(KafkaError::NoMessageReceived)
    }
    #[allow(clippy::let_unit_value)]
    #[allow(unused_variables)]
    pub async fn import_from_mq_for_a_message(&mut self) -> Result<(), ()> {
        
        let kafka_analysis_topic = env::var("KAFKA_ANALYSIS_TOPIC").unwrap();
        let git_url_base = env::var("MEGA_BASE_URL").unwrap();
        
        let ImportMessage { kind, message } = match self.consume_message().await {
            Err(_) => {
                //tracing::warn!("No message in Kafka, please check it!");
                return Err(());
            }
            Ok(m) => m,
        };
        let model = match serde_json::from_slice::<repo_sync_model::MessageModel>(
            message.payload().unwrap(),
        ) {
            Ok(m) => Some(m.clone()),
            Err(e) => {
                tracing::info!("Error while deserializing message payload: {:?}", e);
                None
            }
        };
        if model.is_none(){
            return Err(()) ;
        }
        tracing::info!(
            "Received a message, key: '{:?}', payload: '{:?}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
            message.key(),
            model,
            message.topic(),
            message.partition(),
            message.offset(),
            message.timestamp()
        );

        // 早一个offset，防止当前消息没解析完就结束了
        let offset = message.offset();
        self.context.kafka_offset = Some(offset);
        if offset % 2000 == 0 {
            tracing::info!("Reached message offset: {}", offset);
            self.context.print_status().await;
        }
        //from mega
        let url = Url::parse(&model.unwrap().db_model.mega_url).unwrap();
        let mega_url_suffix = url.path().to_string();
        
        let clone_crates_dir =
            env::var("NEW_CRATES_DIR").unwrap_or_else(|_| CLONE_CRATES_DIR.to_string());
        let split_crates_dir =
            env::var("SPLIT_CRATES_DIR").unwrap_or_else(|_| CLONE_CRATES_DIR.to_string());
        //changes clone_or_not_clone
        let git_url = {
            let git_url_base = Url::parse(&git_url_base)
                .unwrap_or_else(|_| panic!("Failed to parse mega url base: {}", &git_url_base));
            git_url_base
                .join(&mega_url_suffix)
                .expect("Failed to join url path")
        };
        
        let namespace = extract_namespace(git_url.as_ref()).expect("Failed to parse URL");
        let behind_dir = extract_path_from_segment(git_url.as_ref(),"crates").expect("Failed to parse behind_dir");
        let path = PathBuf::from(&clone_crates_dir).join(behind_dir.clone());

        //changes
        if !path.is_dir() {
            //if user_upload, no clone
            tracing::info!("dir {} not exist", path.to_str().unwrap().to_string());
            let clone_start_time = Instant::now();
            let local_repo_path = match self
                .context
                .clone_a_repo_by_url(&clone_crates_dir, &git_url_base, &mega_url_suffix)
                .await
            {
                Ok(x) => x,
                _ => {
                    tracing::error!("Failed to clone repo {}", mega_url_suffix);
                    return Err(());
                }
            };
            let clone_need_time = clone_start_time.elapsed();
            tracing::trace!("clone need time: {:?}", clone_need_time);
            
            let new_versions = self
                .context
                .parse_a_local_repo_and_return_new_versions(local_repo_path, mega_url_suffix)
                .await
                .unwrap();
        } else {
            tracing::info!("dir {} already exist", path.to_str().unwrap().to_string());
            let git_dir = path.join(".git");
            if git_dir.is_dir() {
                tracing::info!("Performing git pull for repository: {}", path.display());
                let pull_result = std::process::Command::new("git")
                    .arg("pull")
                    .current_dir(&path)
                    .status()
                    .map_err(|e| {
                        tracing::error!("Failed to execute git pull: {}", e);
                    });

                if let Ok(status) = pull_result {
                    if !status.success() {
                        tracing::warn!("git pull returned non-zero exit status for {}", path.display());
                    }
                    else{
                        tracing::info!("git pull success");
                    }
                }
            } else {
                tracing::warn!("{} is not a git repository, skipping pull", path.display());
            }
            let insert_time = Instant::now();
            insert_namespace_by_repo_path(path.to_str().unwrap().to_string(), namespace.clone());
            let insert_need_time = insert_time.elapsed();
            tracing::trace!(
                "insert_namespace_by_repo_path need time: {:?}",
                insert_need_time
            );
            
            let new_versions = self
                .context
                .parse_a_local_repo_and_return_new_versions(path, mega_url_suffix)
                .await
                .unwrap();
        } //changes
        tracing::info!("Finish to import from a message!");
        Ok(())
    }

    pub async fn save_checkpoint(&mut self) -> Result<(), Box<dyn Error>> {
        tracing::info!("Saving checkpoint...");
        let checkpoint_dir =
            env::var("CHECKPOINT_DIR").unwrap_or_else(|_| "./checkpoints".to_string());
        tokio::fs::create_dir_all(&checkpoint_dir).await?;

        // 保存二进制checkpoint (如果文件存在会覆盖)
        let checkpoint_path = format!("{checkpoint_dir}/latest.json");
        if tokio::fs::try_exists(&checkpoint_path).await? {
            tokio::fs::remove_file(&checkpoint_path).await?;
        }
        tracing::info!("Saving checkpoint to {}", checkpoint_path);
        self.context.save_to_file(&checkpoint_path).await?;

        // 保存人类可读的摘要 (如果文件存在会覆盖)
        let summary_path = format!("{checkpoint_dir}/summary.txt");
        if tokio::fs::try_exists(&summary_path).await? {
            tokio::fs::remove_file(&summary_path).await?;
        }
        tracing::info!("Saving summary to {}", summary_path);
        tokio::fs::write(summary_path, self.context.format_status()).await?;

        tracing::info!("Checkpoint saved to {}", checkpoint_path);

        Ok(())
    }
    pub async fn export_version(
        &self,
        repo_path: &str,
        oid: &Oid,
        output_path: &str,
        folder_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(e) => {
                tracing::info!("Failed to open repository: {}", e);
                return Err(e.into());
            }
        };
        let output_folder = Path::new(output_path).join(folder_name);
        if output_folder.exists() {
            tokio::fs::remove_dir_all(&output_folder).await?;
        }
        tokio::fs::create_dir_all(&output_folder).await?;
        let tree = repo
            .find_object(*oid, Some(ObjectType::Commit))?
            .peel_to_commit()?
            .tree()
            .unwrap();
        tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            let entry_path = if let Some(name) = entry.name() {
                PathBuf::from(root).join(name)
            } else {
                return 1;
            };

            let output_file_path = output_folder.join(entry_path);
            if entry.kind() == Some(ObjectType::Blob) {
                let blob = entry
                    .to_object(&repo)
                    .and_then(|obj| obj.peel_to_blob())
                    .unwrap();
                if let Some(parent) = output_file_path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(output_file_path, blob.content()).unwrap();
            }
            0
        })?;
        Ok(())
    }
    #[allow(clippy::manual_flatten)]
    pub async fn export_tags(
        &self,
        repo_path: &str,
        output_dir: &str,
        namespace: String,
        crate_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(e) => {
                tracing::info!("Failed to open repository: {}", e);
                return Err(e.into());
            }
        };

        let mut tags = Vec::new();
        if let Ok(tag_names) = repo.tag_names(None) {
            for tag_name in tag_names.iter().flatten() {
                if let Ok(tag_object) = repo.revparse_single(tag_name) {
                    let oid = match tag_object.kind() {
                        Some(ObjectType::Tag) => tag_object.as_tag().unwrap().target_id(),
                        Some(ObjectType::Commit) => tag_object.id(),
                        _ => {
                            continue;
                        }
                    };
                    tags.push((
                        tag_name
                            .replace("/", "_")
                            .replace("\\", "_")
                            .replace(":", "_"),
                        oid,
                    ));
                }
            }
        }
        if !tags.is_empty() {
            let real_output_dir = output_dir.to_string() + "/" + &namespace;
            for (version_name, oid) in &tags {
                let real_version_name = crate_name.clone() + "-" + version_name;
                match self
                    .export_version(repo_path, oid, real_output_dir.as_str(), &real_version_name)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => tracing::info!("Failed to export version {}: {}", version_name, e),
                }
            }
        }
        Ok(())
    }
}

/// 根据环境变量初始化kafka handler
/// KAFKA_CONSUMER_GROUP_ID 会根据测试or部署来设置
/// 详情见 https://github.com/crates-pro/private_docs/discussions/1#discussioncomment-12032278
async fn init_kafka_handler() -> Result<(KafkaHandler, KafkaHandler, KafkaHandler), KafkaError> {
    let kafka_broker = env::var("KAFKA_BROKER").unwrap();
    let consumer_group_id = env::var("KAFKA_CONSUMER_GROUP_ID").unwrap();
    tracing::info!("Kafka parameters: {},{}", kafka_broker, consumer_group_id);
    // 创建三个kafka handler
    // Data from Mega
    let import_handler = KafkaHandler::new_consumer(
        &kafka_broker,
        &consumer_group_id,
        &env::var("KAFKA_ANALYSIS_TEST_TOPIC").unwrap(),
    )
    .expect("Invalid import kafka handler");

    // Data from user-uploading
    let user_import_handler = KafkaHandler::new_consumer(
        &kafka_broker,
        &consumer_group_id,
        &env::var("KAFKA_USER_IMPORT_TOPIC").unwrap(),
    )
    .expect("Invalid import kafka handler");

    // sending for analysis
    let sender_handler =
        KafkaHandler::new_producer(&kafka_broker).expect("Invalid import kafka handler");
    Ok((import_handler, user_import_handler, sender_handler))
}

/// internal structure,
/// a context for repo parsing and importing.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ImportContext {
    pub dont_clone: bool,

    // data to write into
    /// vertex
    pub programs: Vec<Program>,

    pub libraries: Vec<Library>,
    pub applications: Vec<Application>,
    pub library_versions: Vec<LibraryVersion>,
    pub application_versions: Vec<ApplicationVersion>,
    pub versions: Vec<Version>,
    //pub max_versions: Arc<Mutex<HashMap<String, String>>>,
    pub licenses: Vec<Licenses>,
    /// edge
    has_lib_type: Vec<HasType>,
    has_app_type: Vec<HasType>,

    lib_has_version: Vec<HasVersion>,
    app_has_version: Vec<HasVersion>,

    lib_has_dep_version: Vec<HasDepVersion>,
    app_has_dep_version: Vec<HasDepVersion>,

    pub depends_on: Vec<DependsOn>,

    /// help is judge whether it is a new program
    program_memory: HashSet<model::general_model::Program>,
    /// help us judge whether it is a new version
    version_memory: HashSet<model::general_model::Version>,

    pub version_updater: VersionUpdater,

    // 新增字段保存 Kafka offset
    #[serde(default)]
    pub kafka_offset: Option<i64>,
}

impl ImportContext {
    /// Import data from mega
    /// It first clone the repositories locally from mega
    pub async fn compare_versions(a: &str, b: &str) -> Result<std::cmp::Ordering, Box<dyn Error>> {
        let parse_version = |version: &str| {
            let mut parts = version.splitn(2, ['+', '-']); // 分割主版本和附加段
            let version_part = parts.next().unwrap_or("");
            let build_metadata = parts.next(); // 可能的构建元数据或预发行信息

            let version_numbers: Vec<i32> = version_part
                .split('.')
                .map(|v| v.parse::<i32>().unwrap_or(0)) // 解析每个部分
                .collect();

            (version_numbers, build_metadata.is_some())
        };

        let (a_parts, a_has_metadata) = parse_version(a);
        let (b_parts, b_has_metadata) = parse_version(b);

        let max_length = std::cmp::max(a_parts.len(), b_parts.len());

        for i in 0..max_length {
            let a_part = a_parts.get(i).unwrap_or(&0);
            let b_part = b_parts.get(i).unwrap_or(&0);
            let ord = a_part.cmp(b_part);
            if ord != std::cmp::Ordering::Equal {
                return Ok(ord);
            }
        }

        // 版本部分相等，考虑构建元数据
        if a_has_metadata && !b_has_metadata {
            return Ok(std::cmp::Ordering::Less); // A 有构建元数据，B 没有，A较小
        } else if !a_has_metadata && b_has_metadata {
            return Ok(std::cmp::Ordering::Greater); // A 没有构建元数据，B 有，A较大
        }

        Ok(std::cmp::Ordering::Equal)
    }
    pub async fn update_max_version(&mut self) -> Result<String, Box<dyn Error>> {
        let tmp_max_versions: Arc<Mutex<HashMap<String, String>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let mut tasks = vec![];
        for nv in self.versions.clone() {
            let tmp_max_versions = Arc::clone(&tmp_max_versions);
            let name_and_version = nv.name_and_version;
            let task = tokio::spawn(async move {
                if let Some((name, version)) = name_and_version.split_once('/') {
                    let mut tmp_max_versions2 = tmp_max_versions.lock().await;

                    // 获取或插入当前版本
                    let entry = tmp_max_versions2
                        .entry(name.to_string())
                        .or_insert(version.to_string());

                    // 进行版本比较
                    match ImportContext::compare_versions(entry, version).await {
                        Ok(ordering) => {
                            // 仅在新的版本大于当前版本时更新
                            if ordering == std::cmp::Ordering::Less {
                                *entry = version.to_string();
                            }
                        }
                        Err(_) => {
                            //eprintln!("Error comparing versions for {}", name);
                        }
                    }
                }
            });
            tasks.push(task);
        }
        for task in tasks {
            let _ = task.await; // 处理结果（如果需要）
        }
        let get_max_versions = tmp_max_versions.lock().await;
        for (name, version) in get_max_versions.iter() {
            //println!("{}/{}", name, version);
            for p in &mut self.programs {
                if p.name == name.clone() {
                    p.max_version = Some(version.clone());
                    break;
                }
            }
        }
        Ok("".to_string())
    }
    pub async fn max_version(&mut self, v1: &str, v2: &str) -> String {
        let parts1: Vec<&str> = v1.split('.').collect();
        let parts2: Vec<&str> = v2.split('.').collect();
        let max_parts = std::cmp::max(parts1.len(), parts2.len());
        for i in 0..max_parts {
            let num1 = parts1
                .get(i)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            let num2 = parts2
                .get(i)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            #[allow(clippy::comparison_chain)]
            if num1 > num2 {
                return v1.to_string();
            } else if num1 < num2 {
                return v2.to_string();
            }
        }
        v1.to_string()
    }
    async fn parse_a_local_repo_and_return_new_versions(
        &mut self,
        repo_path: PathBuf,
        git_url: String,
    ) -> Result<Vec<model::general_model::VersionWithTag>, String> {
        let mut new_versions = vec![];

        if repo_path.is_dir() && Path::new(&repo_path).join(".git").is_dir() {
            match Repository::open(&repo_path) {
                Err(e) => {
                    tracing::error!("Not a git repo: {:?}, Err: {}", repo_path, e);
                }
                Ok(_) => {
                    // It'a a valid git repository. Start to parse it.
                    tracing::info!("Processing repo: {}", repo_path.display());

                    //reset, maybe useless
                    /*let hard_reset_time = Instant::now();
                    hard_reset_to_head(&repo_path)
                        .await
                        .map_err(|x| format!("{:?}", x))?;
                    let hard_reset_need_time = hard_reset_time.elapsed();
                    tracing::info!("hard_reset_to_head need time: {:?}", hard_reset_need_time);*/
                    let all_programs =
                        self.collect_and_filter_programs(&repo_path, &git_url).await;

                    let all_dependencies =
                        self.collect_and_filter_versions(&repo_path, &git_url).await;
                    let proccess_time = Instant::now();
                    //find max_version
                    

                    //
    
                    for (program, has_type, uprogram) in all_programs {
                        self.programs.push(program.clone());
                        match uprogram {
                            UProgram::Library(l) => {
                                self.libraries.push(l.clone());
                            
                                self.has_lib_type.push(has_type.clone());
                                
                            }
                            UProgram::Application(a) => {
                                self.applications.push(a.clone());
                                
                                self.has_app_type.push(has_type.clone());
                                
                            }
                        };

                        // NOTE: memorize program
                        self.program_memory
                            .insert(model::general_model::Program::new(
                                &program.name,
                                &program.mega_url.clone().unwrap(),
                            ));
                    }
                    
                    for dependencies in all_dependencies.clone() {
                        let name = dependencies.crate_name.clone();
                        let version = dependencies.version.clone();
                        let git_url = dependencies.git_url.clone();
                        let tag_name = dependencies.tag_name.clone();
                        
                        new_versions.push(model::general_model::VersionWithTag::new(
                            &name, &version, &git_url, &tag_name,
                        ));

                        // check whether the crate version exists.
                        let (program, uprogram) = match get_program_by_name(&name) {
                            Some((program, uprogram)) => (program, uprogram),
                            None => {
                                // continue, dont parse
                                continue;
                            }
                        };

                        self.version_updater.update_depends_on(&dependencies).await;

                        let has_version = HasVersion {
                            SRC_ID: program.id.clone(),
                            DST_ID: name_join_version(&name, &version), //FIXME: version id undecided
                        };

                        let dep_version = Version {
                            name_and_version: name_join_version(&name, &version),
                        };

                        #[allow(non_snake_case)]
                        let SRC_ID = name_join_version(&name, &version);
                        #[allow(non_snake_case)]
                        let DST_ID = name_join_version(&name, &version);
                        let has_dep_version = HasDepVersion { SRC_ID, DST_ID };

                        let islib = uprogram.index() == 0;
                        if islib {
                            let version = LibraryVersion::new(
                                program.id.clone(),
                                &name.clone(),
                                &version.clone(),
                                "???",
                            );

                            self.library_versions.push(version.clone());
                            
                            self.lib_has_version.push(has_version.clone());
                            
                            self.lib_has_dep_version.push(has_dep_version.clone());
                            
                        } else {
                            let version = ApplicationVersion::new(
                                program.id.clone(),
                                name.clone(),
                                version.clone(),
                            );

                            self.application_versions.push(version.clone());
                            
                            self.app_has_version.push(has_version.clone());
                            
                            self.app_has_dep_version.push(has_dep_version.clone());
                            
                        }
                        self.versions.push(dep_version.clone());
                        

                        //self.depends_on
                        //    .clone_from(&(self.version_updater.to_depends_on_edges().await));

                        // NOTE: memorize version, insert the new version into memory
                        self.version_memory
                            .insert(model::general_model::Version::new(
                                &dependencies.crate_name,
                                &dependencies.version,
                            ));
                    }
                    for dependency in all_dependencies.clone(){
                        if let Some(tprogram) = self.programs.iter_mut()
                            .find(|p| p.name == dependency.clone().crate_name) 
                        {
                            // 找到匹配项，修改其 max_version
                            let tmp_max_version = tprogram.clone().max_version.unwrap_or_else(|| "0.0.0".to_string());
                            
                            match ImportContext::compare_versions(&tmp_max_version, &dependency.version).await {
                                    Ok(ordering) => {
                                        if ordering == std::cmp::Ordering::Less {
                                            tprogram.max_version = Some(dependency.clone().version);
                                        }
                                    }
                                    Err(_) => {
                                        //eprintln!("Error comparing versions for {}", name);
                                    }
                                };
                            tracing::info!("Updated max_version for {} to {}", tprogram.name, dependency.version);
                        } else {
                            // 未找到匹配项时的处理（可选）
                            tracing::warn!("Program {} not found in self.programs",dependency.crate_name);
                        }
                    }
                    
                    let proccess_need_time = proccess_time.elapsed();
                    tracing::info!("Finish processing repo: {}", repo_path.display());
                    tracing::trace!("processing repo need time: {:?}", proccess_need_time);
                }
            }
        } else {
            tracing::error!("{} is not a directory", repo_path.display());
        }
        Ok(new_versions)
    }

    async fn collect_and_filter_programs(
        &mut self,
        repo_path: &Path,
        git_url: &str,
    ) -> Vec<(Program, HasType, UProgram)> {
        tracing::info!("Start to collect_and_filter_programs {:?}", repo_path);
        let collect_time = Instant::now();
        let all_programs: Vec<(Program, HasType, UProgram)> = extract_info_local(
            repo_path.to_path_buf(),
            git_url.to_owned(),
            &mut self.licenses,
        )
        .await
        .into_iter()
        .filter(|(p, _, _)| {
            !self
                .program_memory
                .contains(&model::general_model::Program::new(
                    &p.name,
                    &p.mega_url.clone().unwrap(),
                ))
        })
        .collect();
        let collect_need_time = collect_time.elapsed();
        tracing::info!("Finish to collect_and_filter_programs {:?}", repo_path);
        tracing::trace!(
            "collect_and_filter_programs need time: {:?}",
            collect_need_time
        );
        all_programs
    }
    async fn collect_and_filter_versions(
        &self,
        repo_path: &PathBuf,
        git_url: &str,
    ) -> Vec<version_info::Dependencies> {
        tracing::info!("Start to collect_and_filter_versions {:?}", repo_path);
        let collect_time = Instant::now();
        // get all versions and dependencies
        // filter out new versions!!!
        let all_dependencies: Vec<version_info::Dependencies> = self
            .parse_all_versions_of_a_repo(repo_path, git_url)
            .await
            .into_iter()
            .filter(|x| {
                !self
                    .version_memory
                    .contains(&model::general_model::Version::new(
                        &x.crate_name,
                        &x.version,
                    ))
            })
            //.filter(|x| semver::Version::parse(&x.version).is_ok())
            .collect();
        let collect_need_time = collect_time.elapsed();
        tracing::info!("Finish to collect_and_filter_versions {:?}", repo_path);
        tracing::trace!(
            "collect_and_filter_versions need time: {:?}",
            collect_need_time
        );
        all_dependencies
    }

    async fn normalize(&mut self) {
        self.depends_on
            .clone_from(&(self.version_updater.to_depends_on_edges().await));
    }

    /// write data base into tugraph import files
    pub async fn write_tugraph_import_files(&mut self) {
        
        self.normalize().await;

        let write_time = Instant::now();
        let tugraph_import_files = PathBuf::from(env::var("TUGRAPH_IMPORT_FILES_PG").unwrap());
        fs::create_dir_all(tugraph_import_files.clone())
            .unwrap_or_else(|e| tracing::error!("Error: {}", e));

        // write into csv
        write_into_csv(
            tugraph_import_files.join("program.csv"),
            self.programs.clone(),
        )
        .unwrap();
        write_into_csv(
            tugraph_import_files.join("library.csv"),
            self.libraries.clone(),
        )
        .unwrap();
        write_into_csv(
            tugraph_import_files.join("application.csv"),
            self.applications.clone(),
        )
        .unwrap();
        write_into_csv(
            tugraph_import_files.join("library_version.csv"),
            self.library_versions.clone(),
        )
        .unwrap();
        write_into_csv(
            tugraph_import_files.join("application_version.csv"),
            self.application_versions.clone(),
        )
        .unwrap();
        write_into_csv(
            tugraph_import_files.join("version.csv"),
            self.versions.clone(),
        )
        .unwrap();
        write_into_csv(
            tugraph_import_files.join("licenses.csv"),
            self.licenses.clone(),
        )
        .unwrap();

        // edge
        let _ = write_into_csv(
            tugraph_import_files.join("has_lib_type.csv"),
            self.has_lib_type.clone(),
        );
        let _ = write_into_csv(
            tugraph_import_files.join("has_app_type.csv"),
            self.has_app_type.clone(),
        );
        let _ = write_into_csv(
            tugraph_import_files.join("lib_has_version.csv"),
            self.lib_has_version.clone(),
        );
        let _ = write_into_csv(
            tugraph_import_files.join("app_has_version.csv"),
            self.app_has_version.clone(),
        );

        let _ = write_into_csv(
            tugraph_import_files.join("lib_has_dep_version.csv"),
            self.lib_has_dep_version.clone(),
        );
        let _ = write_into_csv(
            tugraph_import_files.join("app_has_dep_version.csv"),
            self.app_has_dep_version.clone(),
        );
        let _ = write_into_csv(
            tugraph_import_files.join("depends_on.csv"),
            self.depends_on.clone(),
        );
        
        let write_need_time = write_time.elapsed();
        tracing::trace!("write need time: {:?}", write_need_time);
    }

    pub async fn save_to_file(&mut self, path: &str) -> Result<(), String> {
        self.normalize().await;
        let serialized =
            bincode::serialize(self).map_err(|e| format!("Serialization error: {e}"))?;

        let mut file = File::create(path)
            .await
            .map_err(|e| format!("Failed to create file: {e}"))?;

        file.write_all(&serialized)
            .await
            .map_err(|e| format!("Failed to write to file: {e}"))?;

        Ok(())
    }

    pub async fn load_from_file(path: &str) -> Result<Self, String> {
        tracing::info!("Start to load from file: {}", path);

        let content = tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read file: {e}"))?;

        let context: ImportContext =
            bincode::deserialize(&content).map_err(|e| format!("Deserialization error: {e}"))?;
        tracing::info!(
            "Context loaded successfully, there are {} programs",
            context.programs.len()
        );
        Ok(context)
    }

    fn format_status(&self) -> String {
        format!(
            "Checkpoint Summary:\n\
             Time: {}\n\
             Kafka Offset: {}\n\
             \n\
             Collection Sizes:\n\
             - Programs: {}\n\
             - Libraries: {}\n\
             - Applications: {}\n\
             - Library Versions: {}\n\
             - Application Versions: {}\n\
             - Versions: {}\n\
             - Licenses: {}\n\
             \n\
             Memory Sets:\n\
             - Program Memory: {}\n\
             - Version Memory: {}\n\
             \n\
             Edge Collections:\n\
             - Has Lib Type: {}\n\
             - Has App Type: {}\n\
             - Lib Has Version: {}\n\
             - App Has Version: {}\n\
             - Lib Has Dep Version: {}\n\
             - App Has Dep Version: {}\n\
             - Depends On: {}\n",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            self.kafka_offset.unwrap_or(-1),
            self.programs.len(),
            self.libraries.len(),
            self.applications.len(),
            self.library_versions.len(),
            self.application_versions.len(),
            self.versions.len(),
            self.licenses.len(),
            self.program_memory.len(),
            self.version_memory.len(),
            self.has_lib_type.len(),
            self.has_app_type.len(),
            self.lib_has_version.len(),
            self.app_has_version.len(),
            self.lib_has_dep_version.len(),
            self.app_has_dep_version.len(),
            self.depends_on.len(),
        )
    }

    pub async fn print_status(&mut self) {
        self.normalize().await;
        tracing::info!("{}", self.format_status());
    }

    pub async fn import_from_env_vars(&mut self) -> Result<(), Box<dyn Error>> {
        // 从环境变量获取配置
        let lgraph_import_path = env::var("LGRAPH_IMPORT_PATH").unwrap();
            
        let config_path = env::var("LGRAPH_CONFIG_PATH").unwrap();
            
        let db_dir = env::var("LGRAPH_DB_DIR").unwrap();
            
        let graph_name = env::var("TUGRAPH_CRATESPRO_DB").unwrap();
            
        let overwrite = true;

        // 验证lgraph_import工具是否存在
        if !Path::new(&lgraph_import_path).exists() {
            return Err(format!("lgraph_import工具不存在: {lgraph_import_path}").into());
        }
        
        if !Path::new(&lgraph_import_path).is_file() {
            return Err(format!("{lgraph_import_path} 不是有效的文件").into());
        }

        // 验证配置文件是否存在
        if !Path::new(&config_path).exists() {
            return Err(format!("配置文件不存在: {config_path}").into());
        }
        
        if !Path::new(&config_path).is_file() {
            return Err(format!("{config_path} 不是有效的文件").into());
        }

        // 确保数据库目录存在
        let db_path = Path::new(&db_dir);
        if !db_path.exists() {
            std::fs::create_dir_all(db_path)?;
            println!("创建数据库目录: {db_dir}");
        }

        // 构建异步命令
        let mut cmd = Command::new(&lgraph_import_path);

        // 添加命令参数
        cmd.arg("-c").arg(&config_path)
        .arg("--dir").arg(&db_dir)
        .arg("--graph").arg(&graph_name);
        
        // 添加覆盖参数
        if overwrite {
            cmd.arg("--overwrite").arg("true");
        }
        cmd.arg("--continue_on_error").arg("true");
        // 异步执行命令并获取输出
        let output = cmd.output().await?;

        // 处理命令执行结果
        if output.status.success() {
            //println!("数据导入成功！");
            
            // 打印工具的标准输出
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.is_empty() {
                println!("工具输出:\n{stdout}");
            }
            Ok(())
        } else {
            // 处理错误情况
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            tracing::info!("数据导入失败！");
            tracing::info!("退出代码: {}", output.status.code().unwrap_or(-1));
            
            if !stdout.is_empty() {
                tracing::info!("标准输出:\n{}", stdout);
            }
            
            if !stderr.is_empty() {
                tracing::info!("错误信息:\n{}", stderr);
            }
            Err("lgraph_import命令执行失败".into())
        }
    }
}

impl ImportContext {
    #[allow(unused)]
    fn calculate_memory_usage(&self) -> String {
        use std::fmt::Write;
        use std::mem;
        let mut output = String::new();
        let bytes_per_gb = 1_073_741_824; // 1024^3

        let fields = [
            (
                "Programs",
                self.programs.capacity(),
                mem::size_of::<Program>(),
            ),
            (
                "Libraries",
                self.libraries.capacity(),
                mem::size_of::<Library>(),
            ),
            (
                "Applications",
                self.applications.capacity(),
                mem::size_of::<Application>(),
            ),
            (
                "LibraryVersions",
                self.library_versions.capacity(),
                mem::size_of::<LibraryVersion>(),
            ),
            (
                "ApplicationVersions",
                self.application_versions.capacity(),
                mem::size_of::<ApplicationVersion>(),
            ),
            (
                "Versions",
                self.versions.capacity(),
                mem::size_of::<Version>(),
            ),
            (
                "HasLibType",
                self.has_lib_type.capacity(),
                mem::size_of::<HasType>(),
            ),
            (
                "HasAppType",
                self.has_app_type.capacity(),
                mem::size_of::<HasType>(),
            ),
            (
                "LibHasVersion",
                self.lib_has_version.capacity(),
                mem::size_of::<HasVersion>(),
            ),
            (
                "AppHasVersion",
                self.app_has_version.capacity(),
                mem::size_of::<HasVersion>(),
            ),
            (
                "LibHasDepVersion",
                self.lib_has_dep_version.capacity(),
                mem::size_of::<HasDepVersion>(),
            ),
            (
                "AppHasDepVersion",
                self.app_has_dep_version.capacity(),
                mem::size_of::<HasDepVersion>(),
            ),
            (
                "DependsOn",
                self.depends_on.capacity(),
                mem::size_of::<DependsOn>(),
            ),
            (
                "ProgramMemory",
                self.program_memory.capacity(),
                mem::size_of::<model::general_model::Program>(),
            ),
            (
                "VersionMemory",
                self.version_memory.capacity(),
                mem::size_of::<model::general_model::Version>(),
            ),
        ];

        for (name, capacity, size) in &fields {
            let total_size_bytes = capacity * size;
            let total_size_gb = total_size_bytes as f64 / bytes_per_gb as f64;
            write!(output, " [{name}: {total_size_gb:.4} GB]").unwrap();
        }

        output
    }
}
