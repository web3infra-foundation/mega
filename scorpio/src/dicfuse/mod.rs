use model::GPath;
/// Read only file system for obtaining and displaying monorepo directory information
use reqwest::Client; // Import Response explicitly
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error};
use once_cell::sync::Lazy;
use radix_trie::{self, TrieCommon};
mod model;

const MEGA_TREE_URL: &str = "localhost:8000";//TODO: make it configable

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    name: String,
    path: String,
    content_type: String,
}
#[allow(unused)]
struct DicItem{
    inode:u64,
    name:GPath,
    content_type: ContentType,
}

#[allow(unused)]
enum ContentType {
    File,
    Dictionary,
}
#[allow(unused)]
impl DicItem {
    pub fn new(inode:u64, item:Item) -> Self {
        DicItem {
            inode,
            name: item.name.into(), // Assuming GPath can be created from String
            content_type: match item.content_type.as_str() {
                "file" => ContentType::File,
                "directory" => ContentType::Dictionary,
                _ => panic!("Unknown content type"),
            },
        }
    }
}
#[derive(Serialize, Deserialize, Debug,Default)]
struct ApiResponse {
    req_result: bool,
    data: Vec<Item>,
    err_message: String,
}

// Get Mega dictionary tree from server
#[allow(unused)]
async fn fetch_tree(path: &str) -> Result<ApiResponse, Box<dyn Error>> {
    static CLIENT: Lazy<Client> = Lazy::new(Client::new);
    let client = CLIENT.clone();
    let url = format!("http://{}/api/v1/tree?path={}", MEGA_TREE_URL, path);
    let  resp:ApiResponse = client.get(&url).send().await?.json().await?;
    if resp.req_result {   
        Ok(resp)
    }else{
        todo!();
    }
}

#[allow(unused)]
pub struct Dicfuse{
    next_inode : u64,
    radix_trie: radix_trie::Trie<String,u64>,
    inodes:HashMap<u64,DicItem>,
}
#[allow(unused)]
impl Dicfuse {
    pub fn new() -> Self {
        Dicfuse {
            next_inode: 1,
            radix_trie: radix_trie::Trie::new(),
            inodes: HashMap::new(),
        }
    }
    pub fn import(&self){

    }
    pub fn get_root(&self)->radix_trie::iter::Children<String, u64> {
        let it = self.radix_trie.subtrie("/").unwrap();
        it.children()
    }
    fn lookup(&self,inode :u64)-> Option<GPath>{
        self.inodes.get(&inode).map(|item| item.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore] // This will prevent the test from running by default
    async fn test_fetch_tree_success() {
        let path: &str = "/third-part/mega";

        let result = fetch_tree(path).await.unwrap();
        println!("result: {:?}", result);

    }
}

