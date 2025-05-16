use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;

use callisto::repo_sync_result;
use callisto::sea_orm_active_enums::SyncStatusEnum;
use regex::Regex;
use sea_orm::ActiveModelTrait;
use sea_orm::Set;
use sea_orm::Unchanged;
use url::Url;
use walkdir::WalkDir;

use crate::util;

pub async fn add_and_push_to_remote(workspace: PathBuf) {
    let conn = util::db_connection().await;
    let satellite = observatory::facilities::Satellite::new(
        env::var("KAFKA_BROKER").unwrap().as_str(),
        env::var("KAFKA_TOPIC").unwrap().as_str(),
    );
    let re = Regex::new(r"https://github\.com/[^\s]+").unwrap();
    for entry in WalkDir::new(workspace)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() && entry.depth() == 2 {
            if let Err(err) = env::set_current_dir(entry.path()) {
                eprintln!("Error changing directory: {}", err);
                exit(1);
            }

            let crate_name = entry.file_name().to_str().unwrap().to_owned();
            let mut record = crate::get_record(&conn, &crate_name).await;
            if record.status == Unchanged(SyncStatusEnum::Succeed) {
                tracing::info!("skipping:{:?}", record.crate_name);
                continue;
            }

            let output = Command::new("git")
                .arg("remote")
                .arg("-v")
                .output()
                .unwrap();

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Create a regular expression pattern to match URLs

                let mut capture = re.captures_iter(&stdout);
                if let Some(capture) = capture.next() {
                    let mut url = Url::parse(&capture[0]).unwrap();
                    record.github_url = Set(Some(url.to_string()));
                    url.set_host(Some("localhost")).unwrap();
                    url.set_scheme("http").unwrap();
                    url.set_port(Some(8000)).unwrap();
                    let path = url.path().to_owned();
                    let new_path = format!("/third-part/crates{}", path);
                    url.set_path(&new_path);

                    println!("Found URL: {}", url);
                    record.mega_url = Set(new_path);

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
                    let push_res = Command::new("git").arg("push").arg("nju").output().unwrap();
                    Command::new("git")
                        .arg("push")
                        .arg("nju")
                        .arg("--tags")
                        .output()
                        .unwrap();

                    if push_res.status.success() {
                        record.status = Set(SyncStatusEnum::Succeed);
                        record.err_message = Set(None);
                    } else {
                        record.status = Set(SyncStatusEnum::Failed);
                        record.err_message =
                            Set(Some(String::from_utf8_lossy(&push_res.stderr).to_string()));
                    }
                    record.updated_at = Set(chrono::Utc::now().naive_utc());
                    let res = record.save(&conn).await.unwrap();

                    let kafka_payload: repo_sync_result::Model = res.try_into().unwrap();

                    let handle =
                        satellite.send_message(serde_json::to_string(&kafka_payload).unwrap());
                    // 等待任务完成
                    handle.await.expect("Task failed");
                    println!("Push res: {}", String::from_utf8_lossy(&push_res.stdout));
                    println!("Push err: {}", String::from_utf8_lossy(&push_res.stderr));
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Error running 'git remote -v':\n{}", stderr);
            }
        }
    }
}
