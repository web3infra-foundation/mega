use serde::{Deserialize, Serialize};
use std::fs;

pub mod diff;
pub mod push;
pub mod fetch;
pub mod store;

#[derive(Serialize,Deserialize)]
pub struct  ScorpioManager{
    pub url:String,
    pub workspace:String,
    pub store_path:String,// the path to store init code (or remote code), name is hash value . 
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
        };

        manager.to_toml(TEST_FILE).expect("Failed to write TOML");

        let content = fs::read_to_string(TEST_FILE).expect("Unable to read test file");
        assert!(content.contains("url = \"http://example.com\""));
        assert!(content.contains("mount_path = \"/mnt/example\""));
        assert!(content.contains("path = \"/path/to/work1\""));
        assert!(content.contains("hash = \"hash1\""));
       
    }


    
}