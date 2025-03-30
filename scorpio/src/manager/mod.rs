use bytes::{Bytes, BytesMut};
use ceres::protocol::smart::add_pkt_line_string;
use diff::change;
use mercury::{hash::SHA1, internal::object::{commit::Commit, signature::{Signature, SignatureType}}};
use push::pack;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};
use std::{fs, path::PathBuf, str::FromStr};
use crate::util::scorpio_config;
use crate::manager::diff::add_and_del;

pub mod diff;
pub mod push;
pub mod fetch;
pub mod store;
mod commit;
#[derive(Serialize,Deserialize)]
pub struct  ScorpioManager{
    // pub url:String,
    // pub workspace:String,
    // pub store_path:String,// the path to store init code (or remote code), name is hash value .
    // pub git_author:String,
    // pub git_email:String,
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
        let store_path = scorpio_config::store_path();
        let work_dir = self.select_work(&mono_path)?;
        let path = PathBuf::from(store_path).join(work_dir.hash.clone());
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
        let git_author = scorpio_config::git_author();
        let git_email = scorpio_config::git_email();
        let sign = Signature::new(SignatureType::Author,git_author.to_string(), git_email.to_string());
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

    fn select_work_for_add(&self , mono_path:&str ) ->Result<(&WorkDir, PathBuf), Box<dyn std::error::Error>> {
        println!("Start");
        let workspace = scorpio_config::workspace();
        let workspace = PathBuf::from(workspace);
        let os_mono_path = workspace.join(mono_path); // Construct the full path of mono_path
        println!("Start match");
        println!("os_mono_path = {}", os_mono_path.display());
        
        // Remove this part will cause a security issue, since it will not check the real path.
        // such as:
        /*
            #[test]
            fn test() {
                use std::path::Path;
                use std::fs;

                fs::create_dir_all("/tmp/tmp1746/").unwrap();
                
                let path = Path::new("/usr/bin/../../../../tmp/tmp1746/");
                let path2 = Path::new("/usr");
                
                assert!(path.starts_with(path2));
                assert!(!path.canonicalize().unwrap().starts_with(path2));
            }
        */
        // However, due to the shortcomings of OverlayFs, we have to make compromises.
        /*
        match os_mono_path.canonicalize() {
            // Standardized path
            Ok(path) => {
                println!("len = {}", self.works.len());
                // for works in self.works.iter(){
                for works in self.works.iter() {
                    let os_work_path = workspace.join(&works.path);
                    println!("os_mono_path = {}", path.display());
                    println!("\tos_work_path = {}", os_work_path.display()); 
                    if path.starts_with(os_work_path) {
                        // This allows for partial matches, e.g., if os_mono_path is a subdirectory of works.path
                        println!("Found matching work: {}", works.path);
                        return Ok((works, path));
                    }
                }
                Err(Box::from("WorkDir not found"))
            },
            Err(e) => {
                println!("  Err");
                Err(Box::from(format!("Failed to canonicalize path: {}", e)))
            }
        }
        */
        println!("len = {}", self.works.len());
        // for works in self.works.iter(){
        for works in self.works.iter() {
            let os_work_path = workspace.join(&works.path);
            println!("os_mono_path = {}", os_mono_path.display());
            println!("\tos_work_path = {}", os_work_path.display()); 
            if os_mono_path.starts_with(os_work_path) {
                // This allows for partial matches, e.g., if os_mono_path is a subdirectory of works.path
                println!("Found matching work: {}", works.path);
                return Ok((works, os_mono_path));
            }
        }
        Err(Box::from("WorkDir not found"))
    }

    pub  async fn push_commit(&self,mono_path:&str) ->Result<reqwest::Response, Box<dyn std::error::Error>>{
        
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
        let url = format!("{}/{}/git-receive-pack",base_url,mono_path);
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

    pub async fn mono_add(&self ,mono_path: &str) -> Result<(), Box<dyn std::error::Error>>{
        println!("  mono_path = {}", mono_path);
        let (work_dir, mono_path) = self.select_work_for_add(mono_path)?;
        println!("  select_work_for_add OK");

        let store_path = scorpio_config::store_path();
        println!("  store_path OK");
        let path = PathBuf::from(store_path).join(work_dir.hash.clone());
        println!("  store_path = {}", store_path); // Debugging line to see the store path being used

        let mut dbpath = path.join("store.db");
        let mut upper_path = path.join("upper");

        println!("  dbpath = {:?}", dbpath); // Debugging line to see the database path being used
    
        let db = sled::open(dbpath).unwrap();
        add_and_del(upper_path, path, &db)?;
        println!("OK");
        Ok(())
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
                WorkDir {path:"/path/to/work1".to_string(),hash:"hash1".to_string(), node: 4 },
                WorkDir {path:"/path/to/work2".to_string(),hash:"hash2".to_string(), node: 5 }],
        };

        manager.to_toml(TEST_FILE).expect("Failed to write TOML");

        let content = fs::read_to_string(TEST_FILE).expect("Unable to read test file");
        assert!(content.contains("path = \"/path/to/work1\""));
        assert!(content.contains("hash = \"hash1\""));
       
    }


    
}