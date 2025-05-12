use std::{
    //collections::HashSet,
    env,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    process::{exit, Command},
    str::FromStr,
    sync::Arc,
    time::Instant,
};

use chrono::Utc;
use flate2::bufread::GzDecoder;
use git2::{Repository, Signature};
use observatory::kafka_model::message_model;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set, Unchanged};
use tar::Archive;
use tokio::sync::mpsc;
use url::Url;
use walkdir::WalkDir;

use callisto::{repo_sync_result, sea_orm_active_enums::SyncStatusEnum};

use crate::util;
use observatory::{self};

pub async fn convert_crate_to_repo(workspace: PathBuf) {
    let conn = util::db_connection().await;

    let satellite = observatory::facilities::Satellite::new(
        env::var("KAFKA_BROKER").unwrap().as_str(),
        env::var("KAFKA_TOPIC").unwrap().as_str(),
    );

    let log_file_path = "logfile_3.log";
    let info_log_file_path = "info_1.log";

    let (log_tx, mut log_rx) = mpsc::channel(100);
    let (info_log_tx, mut info_log_rx) = mpsc::channel(100);
    let log_file_path = log_file_path.to_string();
    let info_log_file_path = info_log_file_path.to_string();

    // 启动一个单独的任务来处理日志写入
    tokio::spawn(async move {
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file_path)
            .unwrap();

        while let Some(log_entry) = log_rx.recv().await {
            if let Err(e) = writeln!(log_file, "{}", log_entry) {
                eprintln!("Failed to write to log file: {}", e);
            }
        }
    });

    // 启动一个单独的任务来处理 info 日志写入
    tokio::spawn(async move {
        let mut info_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(info_log_file_path)
            .unwrap();

        while let Some(info_log_entry) = info_log_rx.recv().await {
            if let Err(e) = writeln!(info_log_file, "{}", info_log_entry) {
                eprintln!("Failed to write to info log file: {}", e);
            }
        }
    });

    let mut tasks = vec![];

    for crate_entry in WalkDir::new(workspace)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if crate_entry.path().is_dir() {
            println!("re: {:?}", crate_entry);
            let crate_path = Arc::new(crate_entry.path().to_path_buf()); // 使用 Arc 包装 PathBuf
            let crate_name = Arc::new(
                crate_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            ); // 使用 Arc 包装 String

            let conn_clone = conn.clone();
            let log_tx_clone = log_tx.clone();
            let satellite_clone = satellite.clone();
            let info_log_tx_clone = info_log_tx.clone();
            let crate_path_clone = Arc::clone(&crate_path);
            let crate_name_clone = Arc::clone(&crate_name);

            let task = tokio::spawn(async move {
                if log_tx_clone
                    .send(format!("re: {:?}", crate_path_clone))
                    .await
                    .is_err()
                {
                    eprintln!("Failed to send log: channel closed");
                }
                println!("re: {:?}", crate_path_clone);
                let repo_path = crate_path_clone.join(&*crate_name_clone);

                let record = crate::get_record(&conn_clone, &crate_name_clone).await;
                if record.status == Unchanged(SyncStatusEnum::Succeed) {
                    tracing::info!("skipping:{:?}", record.crate_name);
                    if log_tx_clone
                        .send(format!("skipping: {:?}", record.crate_name))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send log: channel closed");
                    }
                    if info_log_tx_clone
                        .send(format!("info: skipping {:?}", record.crate_name))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send info log: channel closed");
                    }
                    return;
                }

                if repo_path.exists() {
                    println!("repo_path: {}", repo_path.display());
                    if log_tx_clone
                        .send(format!("repo_path: {}", repo_path.display()))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send log: channel closed");
                    }
                    match fs::remove_dir_all(&repo_path) {
                        Ok(()) => (),
                        Err(e) => println!("Failed to remove directory: {}", e),
                    }
                }

                let mut crate_versions: Vec<PathBuf> = WalkDir::new(&*crate_path_clone)
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_type().is_file()
                            && e.path().extension().unwrap_or_default() == "crate"
                    })
                    .map(|entry| entry.path().to_path_buf())
                    .collect();
                crate_versions.sort();

                let start = Instant::now();
                for crate_v in crate_versions {
                    let repo = open_or_make_repo(&repo_path);

                    let start = Instant::now();
                    decompress_crate_file(&crate_v, &crate_path_clone).unwrap_or_else(|e| {
                        eprintln!("{}", e);
                    });
                    let duration = start.elapsed();
                    println!("decompress_crate_file: {:?}", duration.as_millis());
                    if log_tx_clone
                        .send(format!("decompress_crate_file: {:?}", duration.as_millis()))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send log: channel closed");
                    }

                    let uncompress_path = remove_extension(&crate_v);

                    if fs::read_dir(&uncompress_path).is_err() {
                        continue;
                    }

                    if let Err(e) = empty_folder(repo.workdir().unwrap()) {
                        println!("Failed to empty folder: {}", e);
                    }

                    if let Err(e) = copy_all_files(&uncompress_path, repo.workdir().unwrap()) {
                        println!("Failed to copy all files: {}", e);
                    }

                    let version = crate_v
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace(&format!("{}-", crate_name_clone), "")
                        .replace(".crate", "");

                    let start = Instant::now();
                    add_and_commit(&repo, &version, &repo_path);
                    let duration = start.elapsed();
                    println!("add_and_commit: {:?}", duration.as_millis());
                    if log_tx_clone
                        .send(format!("add_and_commit: {:?}", duration.as_millis()))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send log: channel closed");
                    }

                    match fs::remove_dir_all(&uncompress_path) {
                        Ok(()) => (),
                        Err(e) => println!("Failed to remove uncompress_path: {}", e),
                    }
                }
                let duration = start.elapsed();
                println!("total version operation : {:?}", duration.as_millis());
                if log_tx_clone
                    .send(format!(
                        "total version operation: {:?}",
                        duration.as_millis()
                    ))
                    .await
                    .is_err()
                {
                    eprintln!("Failed to send log: channel closed");
                }

                let start = Instant::now();
                if repo_path.exists() {
                    push_to_remote(
                        &conn_clone,
                        &crate_name_clone,
                        &repo_path,
                        record,
                        &satellite_clone,
                    )
                    .await;
                    if info_log_tx_clone
                        .send(format!("info: succeed {:?}", &repo_path))
                        .await
                        .is_err()
                    {
                        eprintln!("Failed to send info log: channel closed");
                    }
                } else {
                    eprintln!("empty crates directory:{:?}", crate_path_clone)
                }
                let duration = start.elapsed();
                println!(
                    "repo_path.exists and push to remote: {:?}",
                    duration.as_millis()
                );
                log_tx_clone
                    .send(format!(
                        "repo_path.exists and push to remote: {:?}",
                        duration.as_millis()
                    ))
                    .await
                    .unwrap();
            });

            tasks.push(task);
        }
    }

    for task in tasks {
        if let Err(e) = task.await {
            eprintln!("Task failed: {:?}", e);
        }
    }
    fn open_or_make_repo(repo_path: &Path) -> Repository {
        match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(_) => {
                println!("Creating a new repository...");
                // Create a new repository
                match Repository::init(repo_path) {
                    Ok(repo) => {
                        println!(
                            "Successfully created a new repository at: {}",
                            repo_path.display()
                        );
                        repo
                    }
                    Err(e) => {
                        panic!("Failed to create a new repository: {}", e);
                    }
                }
            }
        }
    }

    fn add_and_commit(repo: &Repository, version: &str, repo_path: &Path) {
        if let Err(err) = env::set_current_dir(repo_path) {
            eprintln!("Error changing directory: {}", err);
            exit(1);
        }
        // Add all changes in the working directory to the index
        Command::new("git").arg("add").arg("./").output().unwrap();

        // Get the repository index
        let mut index = repo.index().unwrap();

        index.write().unwrap();

        // Create a tree from the index
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        // Get the current HEAD commit (if any)
        let parent_commit = match repo.head() {
            Ok(head) => Some(head.peel_to_commit().unwrap()),
            Err(_) => None,
        };

        // Create a signature
        let sig = Signature::now("Mega", "admin@mega.com").unwrap();

        // Create a new commit
        let commit_id = if let Some(parent) = parent_commit {
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Commit Version: {}", version),
                &tree,
                &[&parent],
            )
            .unwrap()
        } else {
            // Initial commit (no parents)
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Commit Version: {}", version),
                &tree,
                &[],
            )
            .unwrap()
        };

        // Create the tag
        // repo.tag_lightweight(version, &repo.find_object(commit_id, None).unwrap(), false)
        //     .unwrap();
        match repo.tag_lightweight(version, &repo.find_object(commit_id, None).unwrap(), false) {
            Ok(_) => (), // 成功时什么也不做
            Err(e) => match e.code() {
                git2::ErrorCode::Exists => println!("Tag '{}' already exists.", version),
                _ => println!("Failed to create tag: {}", e.message()),
            },
        }
    }

    fn copy_all_files(src: &Path, dst: &Path) -> io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src).unwrap() {
            let entry = entry?;
            let path = entry.path();
            let file_name = match path.file_name() {
                Some(name) => name,
                None => continue,
            };
            let dest_path = dst.join(file_name);

            if path.is_dir() {
                if !path.ends_with(".git") {
                    //copy_all_files(&path, &dest_path).unwrap();
                    match copy_all_files(&path, &dest_path) {
                        Ok(_) => (),
                        Err(e) => eprintln!(
                            "Failed to copy files from {:?} to {:?}: {}",
                            path, dest_path, e
                        ),
                    }
                }
            } else {
                //fs::copy(&path, &dest_path).unwrap();
                if let Err(e) = fs::copy(&path, &dest_path) {
                    println!(
                        "Failed to copy file from {} to {}: {}",
                        path.display(),
                        dest_path.display(),
                        e
                    );
                }
            }
        }
        Ok(())
    }

    fn empty_folder(dir: &Path) -> io::Result<()> {
        for entry in WalkDir::new(dir).min_depth(1).max_depth(1) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                fs::remove_file(path)?;
            } else if path.is_dir() && path.file_name().unwrap() != ".git" {
                fs::remove_dir_all(path)?;
            }
        }
        Ok(())
    }

    async fn push_to_remote(
        conn: &DatabaseConnection,
        crate_name: &str,
        repo_path: &Path,
        mut record: repo_sync_result::ActiveModel,
        satellite: &observatory::facilities::Satellite,
    ) {
        if let Err(err) = env::set_current_dir(repo_path) {
            eprintln!("Error changing directory: {}", err);
            exit(1);
        }

        // let mut url = Url::from_str("http://mono.mega.local:80").unwrap();
        let mut url = Url::from_str(env::var("MEGA_URL").unwrap().as_str()).unwrap();
        let new_path = format!("/third-part/crates/{}", crate_name);
        url.set_path(&new_path);

        Command::new("git")
            .arg("remote")
            .arg("remove")
            .arg("nju")
            .output()
            .unwrap();

        Command::new("git")
            .arg("remote")
            .arg("add")
            .arg("nju")
            .arg(url.to_string())
            .output()
            .unwrap();

        //git push --set-upstream nju master
        let push_res = Command::new("git")
            .arg("push")
            .arg("--set-upstream")
            .arg("nju")
            .arg("main") //改为main
            .output()
            .unwrap();

        Command::new("git")
            .arg("push")
            .arg("nju")
            .arg("--tags")
            .output()
            .unwrap();

        record.mega_url = Set(url.path().to_owned());

        if push_res.status.success() {
            record.status = Set(SyncStatusEnum::Succeed);
            record.err_message = Set(None);
        } else {
            record.status = Set(SyncStatusEnum::Failed);
            record.err_message = Set(Some(String::from_utf8_lossy(&push_res.stderr).to_string()));
        }
        record.updated_at = Set(chrono::Utc::now().naive_utc());
        let res = record.save(conn).await.unwrap();
        let db_model: repo_sync_result::Model = res.try_into().unwrap();
        let kafka_message_model = message_model::MessageModel::new(
            db_model,                              // 数据库 Model
            message_model::MessageKind::Mega,      // 设置 message_kind 为 Mega
            message_model::SourceOfData::Cratesio, // 设置 source_of_data 为 Cratesio
            Utc::now(),                            // 当前时间作为时间戳
            "Extra information".to_string(),       // 设置 extra_field，示例中为一个字符串
        );
        println!("kafka_message{:?}", kafka_message_model);
        let handle = satellite.send_message(serde_json::to_string(&kafka_message_model).unwrap());
        // 等待任务完成
        handle.await.expect("Task failed");
        println!("Push res: {}", String::from_utf8_lossy(&push_res.stdout));
        println!("Push err: {}", String::from_utf8_lossy(&push_res.stderr));
    }

    fn remove_extension(path: &Path) -> PathBuf {
        // Check if the path has an extension
        if let Some(stem) = path.file_stem() {
            // Create a new path without the extension
            if let Some(parent) = path.parent() {
                return parent.join(stem);
            }
        }
        // Return the original path if no extension was found
        path.to_path_buf()
    }

    fn decompress_crate_file(src: &Path, dst: &Path) -> io::Result<()> {
        // Open the source crate file
        let crate_file = File::open(src)?;
        // Create a GzDecoder to handle the gzip decompression
        let tar = GzDecoder::new(BufReader::new(crate_file));
        // Create a tar archive on top of the decompressed tarball
        let mut archive = Archive::new(tar);
        // Unpack the tar archive to the destination directory
        archive.unpack(dst)?;
        Ok(())
    }
}
