use crate::manager::diff::add_and_del;
use crate::util::scorpio_config;
use bytes::{Bytes, BytesMut};
use ceres::protocol::smart::add_pkt_line_string;
use diff::change;
use mercury::{
    hash::SHA1,
    internal::object::{
        commit::Commit,
        signature::{Signature, SignatureType},
    },
};
use push::pack;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, str::FromStr};
use tokio::{fs::File, io::AsyncWriteExt};

mod commit;
pub mod diff;
pub mod fetch;
pub mod push;
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
        let store_path = scorpio_config::store_path();
        let work_dir = self.select_work(&mono_path)?;
        let path = PathBuf::from(store_path).join(work_dir.hash.clone());
        let mut lower = path.clone();
        lower.push("lower");
        let mut upper = path.clone();
        upper.push("upper");
        let mut dbpath = path.clone();
        dbpath.push("tree.db");

        let db = sled::open(dbpath).unwrap();
        let mut trees = Vec::new();
        let mut blobs = Vec::new();
        let root_tree = change(upper, path.clone(), &mut trees, &mut blobs, &db);
        trees.push(root_tree.clone());
        let git_author = scorpio_config::git_author();
        let git_email = scorpio_config::git_email();
        let sign = Signature::new(
            SignatureType::Author,
            git_author.to_string(),
            git_email.to_string(),
        );
        let remote_hash = SHA1::from_str(&work_dir.hash)?;
        let commit = Commit::new(
            sign.clone(),
            sign,
            root_tree.id,
            vec![remote_hash],
            &commit_msg,
        );
        let mut data = BytesMut::new();
        add_pkt_line_string(
            &mut data,
            format!(
                "{} {} {}\0report-status\n",
                work_dir.hash, commit.id, "refs/heads/main"
            ),
        ); //TODO : configable
        data.extend_from_slice(b"0000");
        data.extend(pack(commit.clone(), trees, blobs).await);
        let mut commit_path = path.clone();
        commit_path.push("commit");
        // write back the commit file.
        let mut file = File::create(commit_path).await?;
        file.write_all(&data).await?;
        Ok(commit)
    }

    fn select_work(&self, mono_path: &str) -> Result<&WorkDir, Box<dyn std::error::Error>> {
        for works in self.works.iter() {
            if works.path.eq(&mono_path) {
                return Ok(works);
            }
        }
        Err(Box::from("WorkDir not found"))
    }

    fn select_work_for_add(
        &self,
        mono_path: &str,
    ) -> Result<(&WorkDir, PathBuf), Box<dyn std::error::Error>> {
        for works in self.works.iter() {
            // This allows for partial matches, e.g., if os_mono_path is a subdirectory of works.path
            match PathBuf::from(mono_path).strip_prefix(&works.path) {
                Ok(real_path) => return Ok((works, real_path.to_path_buf())),
                Err(_) => (),
            }
        }
        Err(Box::from("WorkDir not found"))
    }

    pub async fn push_commit(
        &self,
        mono_path: &str,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
        let work_dir = self.select_work(mono_path).unwrap(); // TODO : deal with error.
        let store_path = scorpio_config::store_path();
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
        let base_url = scorpio_config::base_url();
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
        let (work_dir, mono_path) = self.select_work_for_add(mono_path)?;
        let store_path = scorpio_config::store_path();
        let path = PathBuf::from(store_path).join(work_dir.hash.clone());

        // Since index.db is the private space of the sled database,
        // we will combine it with objects to form a new working directory.
        let modifypath = path.join("ModifiedStore");
        let index_dbpath = modifypath.join("index.db");
        let upper_path = path.join("upper");
        let real_path = upper_path.join(&mono_path);

        // In the Upper path, we can safely use the canonicalize function
        // to standardized path.
        match real_path.canonicalize() {
            // Preventing Directory Traversal Vulnerabilities
            Ok(path) => match path.starts_with(upper_path) {
                true => {
                    let index_db = sled::open(index_dbpath).unwrap();
                    add_and_del(path, modifypath, &index_db)?;
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

    const TEST_FILE: &str = "test_config.toml";

    #[test]
    fn test_from_toml() {
        let toml_content = r#"
            works = [{ path = "/path/to/work1", hash = "hash1", node = 1}]
        "#;

        fs::write(TEST_FILE, toml_content).expect("Unable to write test file");

        let manager = ScorpioManager::from_toml(TEST_FILE).expect("Failed to parse TOML");
        assert_eq!(manager.works.len(), 1);
        assert_eq!(manager.works[0].path, "/path/to/work1");
        assert_eq!(manager.works[0].hash, "hash1");
    }

    #[test]
    fn test_to_toml() {
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

        manager.to_toml(TEST_FILE).expect("Failed to write TOML");

        let content = fs::read_to_string(TEST_FILE).expect("Unable to read test file");
        assert!(content.contains("path = \"/path/to/work1\""));
        assert!(content.contains("hash = \"hash1\""));
    }
}
