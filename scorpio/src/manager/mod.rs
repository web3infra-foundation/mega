use crate::manager::store::TempStoreArea;
use crate::util::config;
use add::add_and_del;
use commit::commit_core;
use fs_extra::dir::{copy, CopyOptions};
use mercury::{
    hash::SHA1,
    internal::object::{
        commit::Commit,
        signature::{Signature, SignatureType},
    },
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{fs, path::PathBuf, str::FromStr};

pub mod add;
pub mod commit;
pub mod diff;
pub mod fetch;
pub mod push;
pub mod reset;
pub mod status;
pub mod store;
#[derive(Serialize, Deserialize)]
pub struct ScorpioManager {
    // pub url:String,
    // pub workspace:String,
    // pub store_path:String,// the path to store init code (or remote code), name is hash value .
    // pub git_author:String,
    // pub git_email:String,
    pub works: Vec<WorkDir>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct WorkDir {
    pub path: String,
    pub node: u64,
    pub hash: String,
}

#[allow(unused)]
impl ScorpioManager {
    pub fn from_toml(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let manager: ScorpioManager = toml::de::from_str(&content)?;
        Ok(manager)
    }

    pub fn to_toml(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::ser::to_string(self)?;
        fs::write(file_path, content)?;
        Ok(())
    }
    pub async fn mono_commit(
        &self,
        mono_path: String,
        commit_msg: String,
    ) -> Result<Commit, Box<dyn std::error::Error>> {
        let store_path = config::store_path();
        let work_dir = self.select_work(&mono_path)?;
        let work_path = PathBuf::from(store_path).join(work_dir.hash.clone());
        let old_dbpath = work_path.join("tree.db");
        let new_dbpath = work_path.join("new_tree.db");
        let objectspath = work_path.join("objects");
        let commitpath = work_path.join("commit");
        let modified_path = work_path.join("modifiedstore");
        let tempstorage_path = modified_path.join("objects");

        println!("old_dbpath = {}", old_dbpath.display());
        println!("new_dbpath = {}", new_dbpath.display());

        let _ = fs::remove_dir_all(&objectspath);
        if tempstorage_path.exists() {
            println!("tempstorage_path = {}, Copy", tempstorage_path.display());
            let mut options = CopyOptions::new();
            options.copy_inside = true;
            copy(&tempstorage_path, &objectspath, &options)?;
        }

        let old_tree_db = sled::open(old_dbpath)?;
        let new_tree_db = sled::open(new_dbpath)?;
        let temp_store_area = TempStoreArea::new(&modified_path)?;
        let old_root_path = PathBuf::from(mono_path);

        let git_author = config::git_author();
        let git_email = config::git_email();
        let sign = Signature::new(
            SignatureType::Author,
            git_author.to_string(),
            git_email.to_string(),
        );

        // For the sake of logical integrity and emergency response
        // capabilities, Parent Commit is checked first.
        let parent_commit = fs::read_to_string(&commitpath)?;
        let regex_rule = Regex::new(r#"tree: (?<parent_hash>[0-9a-z]{40})"#).unwrap();
        let parent_hash = match regex_rule.captures(&parent_commit) {
            Some(parent_info) => vec![SHA1::from_str(&parent_info["parent_hash"])?],
            None => return Err(Box::from("Parent hash not found in commit file")),
        };

        println!("\x1b[34m[START]\x1b[0m");
        let main_tree_hash = commit_core(
            (&old_tree_db, &new_tree_db),
            &temp_store_area,
            &old_root_path,
        )?;
        println!("\x1b[34m[DONE]\x1b[0m");

        println!("   [\x1b[33mDEBUG\x1b[0m] commit.author = {}", sign.name);
        println!("   [\x1b[33mDEBUG\x1b[0m] commit.committer = {}", sign.name);
        println!(
            "   [\x1b[33mDEBUG\x1b[0m] commit.tree_id = {}",
            main_tree_hash._to_string()
        );
        println!(
            "   [\x1b[33mDEBUG\x1b[0m] commit.parent_commit_ids = {}",
            parent_hash[0]
        );
        println!("   [\x1b[33mDEBUG\x1b[0m] commit.message = {}", commit_msg);

        let commit = Commit::new(sign.clone(), sign, main_tree_hash, parent_hash, &commit_msg);

        let mut commit_file = std::fs::File::create(&commitpath)?;
        commit_file.write_all(commit.to_string().as_bytes())?;

        Ok(commit)
    }

    /// Extracts and returns the corresponding workspace for the provided `mono_path`.
    ///
    /// This function iterates over the manager's work directories and selects the one whose path
    /// is either exactly equal to `mono_path` or is a prefix of `mono_path`. In other words, it
    /// finds the workspace that best matches the given path.
    ///
    /// # Parameters
    ///
    /// - `mono_path`: A string slice representing the path to match against the work directories.
    ///
    /// # Returns
    ///
    /// - `Ok(&WorkDir)` if a matching workspace is found.
    /// - `Err("WorkDir not found")` otherwise.
    fn select_work(&self, mono_path: &str) -> Result<&WorkDir, Box<dyn std::error::Error>> {
        for works in self.works.iter() {
            if mono_path.starts_with(&works.path) || mono_path.eq(&works.path) {
                return Ok(works);
            }
        }
        Err(Box::from("WorkDir not found"))
    }

    pub async fn push_commit(
        &self,
        mono_path: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let store_path = config::store_path();
        let work_dir = self.select_work(mono_path)?;
        let work_path = PathBuf::from(store_path).join(work_dir.hash.clone());
        let modified_path = work_path.join("modifiedstore");
        let temp_store_area = TempStoreArea::new(&modified_path)?;
        println!("OK1");
        let base_url = config::base_url();
        let url = format!("{}/{}/git-receive-pack", base_url, mono_path);

        println!("START");
        let res = push::push(&work_path, &url, &temp_store_area.index_db).await?;
        println!("END");
        Ok(res)
    }
    /*
    pub async fn push_commit(
        &self,
        mono_path: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let work_dir = self.select_work(mono_path)?; // TODO : deal with error.
        let store_path = config::store_path();
        let mut path = store_path.to_string();
        path.push_str(&work_dir.hash);
        path.push_str("commit");

        // check path is exist
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            eprintln!("Path does not exist: {}", path);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", path),
            )));
        }
        // read the file as the body to send
        let commit_data = tokio::fs::read(&path).await?;

        // Send Commit data to remote mono.
        let base_url = config::base_url();
        let url = format!("{}/{}/git-receive-pack", base_url, mono_path);
        let client = reqwest::Client::new();
        client
            .post(&url)
            .header("Content-Type", "application/x-git-receive-pack-request")
            .body(Bytes::from(commit_data))
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
    */

    pub fn check_before_mount(&self, mono_path: &str) -> Result<(), String> {
        for work in &self.works {
            if work.path.starts_with(mono_path) || mono_path.starts_with(&work.path) {
                return Err(work.path.clone());
            }
        }
        Ok(())
    }
    /// Iterate through the manager's works to find the specified path's workspace and remove it.
    pub async fn remove_workspace(
        &mut self,
        mono_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(pos) = self.works.iter().position(|work| work.path == mono_path) {
            self.works.remove(pos);
            self.to_toml("config.toml")?;
            Ok(())
        } else {
            Err(Box::from("Workspace not found"))
        }
    }

    pub async fn mono_add(&self, mono_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // The OS path cannot be used, and should be mapped from
        // the FUSE system to the path under Upper.
        // For example, work_dir/1.sh corresponds to upper/1.sh
        //
        // What is needed here is a path relative to Upper, not a path
        // relative to the Workdir.
        let work_dir = self.select_work(mono_path)?;
        let store_path = config::store_path();
        let work_path = PathBuf::from(store_path).join(work_dir.hash.clone());
        let mono_path = PathBuf::from(mono_path)
            .strip_prefix(&work_dir.path)?
            .to_path_buf();

        // Since index.db is the private space of the sled database,
        // we will combine it with objects to form a new working directory.
        let modified_path = work_path.join("modifiedstore");
        let upper_path = work_path.join("upper");
        let real_path = upper_path.join(mono_path);
        println!("real_path = {}", real_path.display());

        // In the Upper path, we can safely use the canonicalize function
        // to standardized path.
        match real_path.canonicalize() {
            // Preventing Directory Traversal Vulnerabilities
            Ok(format_path) => match format_path.starts_with(upper_path) {
                true => {
                    let temp_store_area = TempStoreArea::new(&modified_path)?;
                    println!("\x1b[32m[START]\x1b[0m");
                    add_and_del(&format_path, &work_path, &temp_store_area).await?;
                    println!("\x1b[32m[OK]\x1b[0m");
                    Ok(())
                }
                false => {
                    let e_message = format!("Not allowed path: {}", real_path.display());
                    Err(Box::from(e_message))
                }
            },
            Err(e) => {
                let e_message = format!("Failed to canonicalize path: {}", e);
                Err(Box::from(e_message))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_from_toml() {
        let tmp_file = format!("{}/test_from_toml_1.toml", env::temp_dir().display(),);
        let toml_content = r#"
            works = [{ path = "/path/to/work1", hash = "hash1", node = 1}]
        "#;

        fs::write(&tmp_file, toml_content).expect("Unable to write test file");

        let manager = ScorpioManager::from_toml(&tmp_file).expect("Failed to parse TOML");
        assert_eq!(manager.works.len(), 1);
        assert_eq!(manager.works[0].path, "/path/to/work1");
        assert_eq!(manager.works[0].hash, "hash1");

        fs::remove_file(&tmp_file).ok();
    }

    #[test]
    fn test_to_toml() {
        let tmp_file = format!("{}/test_to_toml_2.toml", env::temp_dir().display(),);
        let manager = ScorpioManager {
            works: vec![
                WorkDir {
                    path: "/path/to/work1".to_string(),
                    hash: "hash1".to_string(),
                    node: 4,
                },
                WorkDir {
                    path: "/path/to/work2".to_string(),
                    hash: "hash2".to_string(),
                    node: 5,
                },
            ],
        };

        manager.to_toml(&tmp_file).expect("Failed to write TOML");

        let content = fs::read_to_string(&tmp_file).expect("Unable to read test file");
        assert!(content.contains("path = \"/path/to/work1\""));
        assert!(content.contains("hash = \"hash1\""));

        fs::remove_file(&tmp_file).ok();
    }
}
