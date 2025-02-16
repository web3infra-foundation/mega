use bytes::{Bytes, BytesMut};
use ceres::protocol::smart::add_pkt_line_string;
use diff::change;
use mercury::{hash::SHA1, internal::object::{commit::Commit, signature::{Signature, SignatureType}}};
use push::pack;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};
use std::{fs, path::PathBuf, str::FromStr};

pub mod diff;
pub mod push;
pub mod fetch;
pub mod store;
mod commit;
#[derive(Serialize,Deserialize)]
pub struct  ScorpioManager{
    pub url:String,
    pub workspace:String,
    pub store_path:String,// the path to store init code (or remote code), name is hash value . 
    pub git_author:String,
    pub git_email:String,
    pub works:Vec<WorkDir>,
}
#[derive(Serialize,Deserialize,Clone)]
pub struct WorkDir{
    pub path:String,
    pub node:u64,
    pub hash:String,
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
    pub async fn mono_commit(&self ,mono_path:String, commit_msg:String) -> Result<Commit, Box<dyn std::error::Error>>{
        let work_dir = self.select_work(&mono_path)?;
        let path = PathBuf::from(self.store_path.clone());
        path.join(work_dir.hash.clone());
        let mut lower  = path.clone();
        lower.push("lower");
        let mut upper  = path.clone();
        upper.push("upper");
        let mut dbpath  = path.clone();
        dbpath.push("tree.db");
    
        let db = sled::open(dbpath).unwrap();
        let mut trees = Vec::new();
        let mut blobs= Vec::new();
        let root_tree = change(upper, path.clone(), &mut trees, &mut blobs, &db);
        trees.push(root_tree.clone());
        let sign = Signature::new(SignatureType::Author,self.git_author.clone(), self.git_email.clone());
        let remote_hash  = SHA1::from_str(&work_dir.hash)?;
        let commit = Commit::new(
            sign.clone(),
            sign, 
            root_tree.id, 
            vec![remote_hash], 
            &commit_msg);
        let mut data = BytesMut::new();
        add_pkt_line_string(&mut data, format!("{} {} {}\0report-status\n",
                                            work_dir.hash,
                                            commit.id,
                                            "refs/heads/main"));//TODO : configable
        data.extend_from_slice(b"0000");
        data.extend(pack(commit.clone(),trees,blobs).await);
        let mut commit_path = path.clone();
        commit_path.push("commit");  
        // write back the commit file.
        let mut file = File::create(commit_path).await?;  
        file.write_all(&data).await?;  
        Ok(commit)
    }

    fn select_work(&self , mono_path:&str ) ->Result<&WorkDir, Box<dyn std::error::Error>> {
        for works in self.works.iter(){
            if works.path.eq(&mono_path){
                return Ok(works);
            }
        }
        Err(Box::from("WorkDir not found"))
        
    }
    pub  async fn push_commit(&self,mono_path:&str) ->Result<reqwest::Response, Box<dyn std::error::Error>>{
        
        let work_dir = self.select_work(mono_path).unwrap(); // 错误处理根据实际需求实现
        let mut path = self.store_path.clone();
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
        let url = format!("{}/{}/git-receive-pack",self.url,mono_path);
        let client = reqwest::Client::new();
        client
            .post(&url)
            .header("Content-Type", "application/x-git-receive-pack-request")
            .body(Bytes::from(commit_data))
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)


    }

    pub fn check_before_mount(&self , mono_path: &str) ->Result<(),String>{
        for work in &self.works {
            if work.path.starts_with(mono_path) || mono_path.starts_with(&work.path) {
                return Err(work.path.clone());
            }
        }
        Ok(())
    }
    /// Iterate through the manager's works to find the specified path's workspace and remove it.
    pub async fn remove_workspace(&mut self, mono_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(pos) = self.works.iter().position(|work| work.path == mono_path) {
            self.works.remove(pos);
            self.to_toml("config.toml")?;
            Ok(())
        } else {
            Err(Box::from("Workspace not found"))
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
            url = "http://example.com"
            mount_path = "/mnt/example"
            works = [{ path = "/path/to/work1", hash = "hash1" }]
        "#;

        fs::write(TEST_FILE, toml_content).expect("Unable to write test file");

        let manager = ScorpioManager::from_toml(TEST_FILE).expect("Failed to parse TOML");
        assert_eq!(manager.url, "http://example.com");
        assert_eq!(manager.workspace, "/mnt/example");
        assert_eq!(manager.works.len(), 1);
        assert_eq!(manager.works[0].path, "/path/to/work1");
        assert_eq!(manager.works[0].hash, "hash1");

    }

    #[test]
    fn test_to_toml() {
        let manager = ScorpioManager {
            url: "http://example.com".to_string(),
            workspace: "/mnt/example".to_string(),
            works: vec![
                WorkDir {path:"/path/to/work1".to_string(),hash:"hash1".to_string(), node: 4 },
                WorkDir {path:"/path/to/work2".to_string(),hash:"hash2".to_string(), node: 5 }],
            store_path: "/path/to/lower".to_string(),
            git_author: "MEGA".to_string(),
            git_email:  "admin@mega.com".to_string(),
        };

        manager.to_toml(TEST_FILE).expect("Failed to write TOML");

        let content = fs::read_to_string(TEST_FILE).expect("Unable to read test file");
        assert!(content.contains("url = \"http://example.com\""));
        assert!(content.contains("mount_path = \"/mnt/example\""));
        assert!(content.contains("path = \"/path/to/work1\""));
        assert!(content.contains("hash = \"hash1\""));
       
    }


    
}