use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    process::{exit, Command},
    str::FromStr,
    time::Instant,
};

use callisto::{repo_sync_result, sea_orm_active_enums::SyncStatusEnum};
use chrono::Utc;
use flate2::bufread::GzDecoder;
use git2::{Repository, Signature};
use observatory::kafka_model::message_model;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set, Unchanged};
use semver::Version;
use tar::Archive;
use url::Url;
use walkdir::WalkDir;

use crate::util;
use observatory::{self};
pub async fn incremental_update() {
    let conn = util::db_connection().await;
    let satellite = observatory::facilities::Satellite::new(
        env::var("KAFKA_BROKER").unwrap().as_str(),
        env::var("KAFKA_TOPIC").unwrap().as_str(),
    );

    // 从日志中读出增量信息
    let download_txt_dir = Path::new("/home/rust/freighter/log/");
    let crate_names: HashSet<String> =
        read_latest_crate_list(download_txt_dir).expect("Failed to get download.txt crates name");

    println!("log_read-done!");
    let work_dir = PathBuf::from("/mnt/crates/freighter/crates");

    for crate_name in crate_names {
        let crate_entry = work_dir.join(&crate_name);
        if crate_entry.exists() && crate_entry.is_dir() {
            // 访问对应的目录
            println!("Accessing directory: {:?}", crate_entry);
            // 在这里添加你需要对目录进行的操作
            println!("re: {:?}", crate_entry);
            let crate_path = crate_entry.as_path();
            let crate_name = crate_name.as_str();
            let repo_path = &crate_path.join(crate_name);

            let record: repo_sync_result::ActiveModel = crate::get_record(&conn, crate_name).await;

            let crate_versions: Vec<PathBuf> = get_sorted_crate_versions(crate_path, crate_name);
            let latest_version = crate_versions
                .last()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(&format!("{}-", crate_name), "")
                .replace(".crate", "");

            let semver_latest_version =
                Version::parse(&latest_version).expect("Failed to parse latest version");
            if record.status == Unchanged(SyncStatusEnum::Succeed) {
                if let Ok(record_version) = Version::parse(record.version.as_ref()) {
                    if record_version >= semver_latest_version {
                        tracing::info!("skipping:{:?}", record.crate_name);
                        continue;
                    }
                } else {
                    tracing::error!("Failed to parse version for record: {:?}", record);
                }
            }

            let start = Instant::now();
            for crate_v in crate_versions {
                process_cratefile_to_repo(&crate_v, crate_path, repo_path, crate_name, &record);
            }
            let duration = start.elapsed();
            println!("total version operation : {:?}", duration.as_millis());

            let start = Instant::now();
            if repo_path.exists() {
                push_to_remote(
                    &conn,
                    crate_name,
                    repo_path,
                    record,
                    &latest_version,
                    &satellite,
                )
                .await;
            } else {
                eprintln!("empty crates directory:{:?}", crate_entry.display());
            }
            let duration = start.elapsed();
            println!(
                "repo_path.exists and push to remote: {:?}",
                duration.as_millis()
            );
        } else {
            println!("Directory does not exist: {:?}", crate_entry);
        }
    }

    fn get_sorted_crate_versions(crate_path: &Path, crate_name: &str) -> Vec<PathBuf> {
        let mut crate_versions: Vec<PathBuf> = WalkDir::new(crate_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && e.path().extension().unwrap_or_default() == "crate"
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();

        crate_versions.sort_by(|a, b| {
            let a_version = a
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix(&format!("{}-", crate_name)))
                .and_then(|s| Version::parse(s).ok());
            let b_version = b
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix(&format!("{}-", crate_name)))
                .and_then(|s| Version::parse(s).ok());
            a_version.cmp(&b_version)
        });

        crate_versions
    }

    fn process_cratefile_to_repo(
        crate_v: &Path,
        crate_path: &Path,
        repo_path: &Path,
        crate_name: &str,
        record: &repo_sync_result::ActiveModel,
    ) {
        let version = &crate_v
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(&format!("{}-", crate_name), "")
            .replace(".crate", "");
        if let (Ok(record_version), Ok(semver_version)) = (
            Version::parse(record.version.as_ref()),
            Version::parse(version),
        ) {
            if semver_version == Version::new(0, 0, 0) {
                if record_version != semver_version {
                    return;
                }
            } else if record_version >= semver_version {
                return;
            }
        } else {
            tracing::error!(
                "Failed to parse version for comparison: record_version = {:?}, version = {:?}",
                record.version,
                version
            );
        }

        let repo = open_or_make_repo(repo_path);

        let start = Instant::now();
        decompress_crate_file(crate_v, crate_path).unwrap_or_else(|e| {
            eprintln!("{}", e);
        });
        let duration = start.elapsed();
        tracing::info!("decompress_crate_file: {:?}", duration.as_millis());

        let uncompress_path = remove_extension(crate_v);

        if fs::read_dir(&uncompress_path).is_err() {
            return;
        }

        if let Err(e) = empty_folder(repo.workdir().unwrap()) {
            tracing::error!("Failed to empty folder: {}", e);
        }

        if let Err(e) = copy_all_files(&uncompress_path, repo.workdir().unwrap()) {
            tracing::error!("Failed to copy all files: {}", e);
        }

        let start = Instant::now();
        add_and_commit(&repo, version, repo_path);
        let duration = start.elapsed();
        tracing::info!("add_and_commit: {:?}", duration.as_millis());

        match fs::remove_dir_all(uncompress_path) {
            Ok(()) => (),
            Err(e) => tracing::error!("Failed to remove uncompress_path: {}", e),
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
        match repo.tag_lightweight(version, &repo.find_object(commit_id, None).unwrap(), false) {
            Ok(_) => (),
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
                    if let Err(e) = copy_all_files(&path, &dest_path) {
                        println!(
                            "Failed to copy file from {} to {}: {}",
                            path.display(),
                            dest_path.display(),
                            e
                        );
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
        version: &String,
        satellite: &observatory::facilities::Satellite,
    ) {
        if let Err(err) = env::set_current_dir(repo_path) {
            eprintln!("Error changing directory: {}", err);
            exit(1);
        }

        let mut url = Url::from_str(env::var("MEGA_URL").unwrap().as_str()).unwrap();
        let new_path = format!("/third-part/crates/{}", crate_name);
        url.set_path(&new_path);

        let push_res = update_git_remote_and_push(&url);

        record.mega_url = Set(url.path().to_owned());

        if push_res.status.success() {
            record.status = Set(SyncStatusEnum::Succeed);
            record.err_message = Set(None);
        } else {
            record.status = Set(SyncStatusEnum::Failed);
            record.err_message = Set(Some(String::from_utf8_lossy(&push_res.stderr).to_string()));
        }
        record.updated_at = Set(chrono::Utc::now().naive_utc());
        record.version = Set(version.to_string());
        let res = record.save(conn).await.unwrap();
        let db_model: repo_sync_result::Model = res.try_into().unwrap();
        let kafka_message_model = message_model::MessageModel::new(
            db_model,                              // 数据库 Model
            message_model::MessageKind::Mega,      // 设置 message_kind 为 Mega
            message_model::SourceOfData::Cratesio, // 设置 source_of_data 为 Cratesio
            Utc::now(),                            // 当前时间作为时间戳
            "Extra information".to_string(),       // 设置 extra_field，示例中为一个字符串
        );
        println!("kafka_message {:?}", kafka_message_model);
        let handle = satellite.send_message(serde_json::to_string(&kafka_message_model).unwrap());
        // 等待任务完成
        handle.await.expect("Task failed");
        println!("Push res: {}", String::from_utf8_lossy(&push_res.stdout));
        println!("Push err: {}", String::from_utf8_lossy(&push_res.stderr));
    }

    fn update_git_remote_and_push(url: &Url) -> std::process::Output {
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

        // 获取当前分支名称
        let branch_output = Command::new("git")
            .arg("symbolic-ref")
            .arg("--short")
            .arg("HEAD")
            .output()
            .expect("Failed to get current branch name");

        let branch_name = std::str::from_utf8(&branch_output.stdout)
            .expect("Failed to parse branch name")
            .trim();

        // 根据当前分支名称执行 git push
        let push_res = Command::new("git")
            .arg("push")
            .arg("--set-upstream")
            .arg("nju")
            .arg(branch_name)
            .output()
            .expect("Failed to push to remote");

        if !push_res.status.success() {
            eprintln!("Failed to push branch: {:?}", push_res);
        }

        let push_tags_res = Command::new("git")
            .arg("push")
            .arg("nju")
            .arg("--tags")
            .output()
            .expect("Failed to push tags to remote");

        if !push_tags_res.status.success() {
            eprintln!("Failed to push tags: {:?}", push_tags_res);
        }

        push_res
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

    /// 从目录中读取最新的 `download_*.txt` 文件，解析出 crate 名集合
    fn read_latest_crate_list(dir: &Path) -> io::Result<HashSet<String>> {
        let mut latest_file: Option<PathBuf> = None;
        let mut latest_timestamp: Option<i64> = None;

        // 遍历目录下的文件
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            //println!("open txt {:?}", path);
            // 检查文件是否符合 `download_*.txt` 命名格式
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                if file_name.starts_with("download_") && file_name.ends_with(".txt") {
                    // 提取时间戳部分
                    let timestamp_str = file_name
                        .trim_start_matches("download_")
                        .trim_end_matches(".txt");

                    let parts: Vec<&str> = timestamp_str.split('_').collect();
                    let date_part = parts[0].replace('-', "");
                    let time_part = parts[1].replace('-', "");

                    // 拼接日期和时间部分
                    let formatted_str = format!("{}{}", date_part, time_part);
                    // 尝试将时间戳转换为数字
                    if let Ok(timestamp) = formatted_str.parse::<i64>() {
                        if latest_timestamp.is_none() || timestamp > latest_timestamp.unwrap() {
                            latest_timestamp = Some(timestamp);
                            latest_file = Some(path);
                        }
                    }
                }
            }
        }

        println!("open txt {:?}", latest_file);
        // 如果找到最新文件，读取文件内容
        if let Some(latest_file_path) = latest_file {
            let file = fs::File::open(latest_file_path)?;
            let reader = BufReader::new(file);
            let mut crate_names = HashSet::new();

            for line in reader.lines() {
                let line = line?;
                crate_names.insert(line);
            }

            Ok(crate_names)
        } else {
            // 如果未找到符合条件的文件，则返回空集合
            Ok(HashSet::new())
        }
    }
}
