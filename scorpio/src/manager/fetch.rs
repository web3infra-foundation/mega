use std::path::{Path, PathBuf};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use axum::async_trait;
use mercury::hash::SHA1;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use reqwest::Client;

use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::time;
use async_recursion::async_recursion;

use crate::manager::store::store_trees;
use crate::util::GPath;
use crate::util::scorpio_config;

use super::{ScorpioManager, WorkDir};

#[allow(unused)]
#[async_trait]
pub trait CheckHash{
    async fn check(&mut self);
    
    async fn fetch<P: AsRef<Path>+ std::marker::Send  >(&mut self,inode:u64,monopath :P)-> WorkDir;
}

#[async_trait]
impl CheckHash for ScorpioManager{
    async fn check(&mut self) {
        
        let mut handlers = Vec::new();

        for work in &mut self.works {
            // if the config hash is null or empty , mean that it's a new config work node path .
            if work.hash.is_empty() {
                let p = GPath::from(work.path.to_string());
                // Get the tree and its hash value, for name dictionary .
                let tree = fetch_tree(&p).await.unwrap();
                work.hash = tree.id.to_string();
                // the lower path is store file path for remote code version .
                let store_path = scorpio_config::store_path();
                let _lower = PathBuf::from(store_path).join(&work.hash).join("lower");
                handlers.push(tokio::spawn(async move { fetch_code(&p, _lower).await }));
            }
        }
        // if have new config path , finish all handlers and write back the config file
        if !handlers.is_empty(){
            for handle in handlers {
                let _ = handle.await;
            }
            //Get config file path from scorpio_config.rs
            let config_file = scorpio_config::config_file();
            let _ = self.to_toml(config_file);

        }

    }
    
    async fn fetch<P: AsRef<Path> + std::marker::Send  >(&mut self,inode:u64,monopath :P) -> WorkDir {
        let path = monopath.as_ref().to_str().unwrap().to_string();
        let p = GPath::from(path);
        // Get the tree and its hash value, for name dictionary .
        let tree = fetch_tree(&p).await.unwrap();
        let workdir = WorkDir{
            path: p.to_string(),
            node:inode,
            hash: tree.id.to_string(),
        };
        //work.hash = tree.id.to_string();
        // the lower path is store file path for remote code version . 
        let store_path = scorpio_config::store_path();
        let _lower = PathBuf::from(store_path).join(&workdir.hash).join("lower");
        fetch_code(&p, _lower).await;
        self.works.push(workdir.clone());
        let config_file = scorpio_config::config_file();
        let _ = self.to_toml(config_file);

        workdir
    }
}

pub async fn fetch<P: AsRef<Path>>(manager:&mut ScorpioManager,inode:u64,monopath :P) -> WorkDir {
    let path = monopath.as_ref().to_str().unwrap().to_string();
    let p = GPath::from(path);
    // Get the tree and its hash value, for name dictionary .
    let tree =fetch_tree(&p).await.unwrap();
    let workdir = WorkDir{
        path: p.to_string(),
        node:inode,
        hash: tree.id.to_string(),
    };
    //work.hash = tree.id.to_string();
    // the lower path is store file path for remote code version . 
    let store_path = scorpio_config::store_path();
    let _lower = PathBuf::from(store_path).join(&workdir.hash).join("lower");
    fetch_code(&p, _lower).await;
    manager.works.push(workdir.clone());
    let config_file = scorpio_config::config_file();
    let _ = manager.to_toml(config_file);
    workdir
}

const BASE_URL : &str = "http://localhost:8000/api/v1/file/tree?path=/";
#[allow(unused)]
#[allow(clippy::blocks_in_conditions)]
async fn worker_thread(
    id:u32,
    root_path:GPath,
    target_path:&Path,
    shared_queue: Arc<Mutex<VecDeque<GPath>>>,
    send_tree :Sender<Tree>,
) {
    let client = Client::new();
    let mut interval = time::interval(Duration::from_millis(50)); 
    let timeout_duration = Duration::from_millis(300);
    loop {
        let path = tokio::select! {
            _ = time::sleep(timeout_duration) => {
                // If timeout and no more tree, finish this thread.
                println!("Timeout occurred while waiting for path");
                break;
            },
            path = async {
                loop{
                    {
                        let mut queue = shared_queue.lock().await;
                        if let Some(pa) = queue.pop_front(){
                            break pa;
                        }
                    }
                }
            } => {
                path
            }
        };
        // deal with  path .
        let url = format!("{}{}", BASE_URL, path);
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            match Tree::try_from(&bytes[..]) {
                                Ok(tree) => {
                                    trace!("ID:{},path:{}",id,path);
                                    send_tree.send(tree.clone()).await;
                                    //trace!("path:{},new tree:{}",path,tree );
                                    for item in tree.tree_items {

                                        let mut subpath = path.clone();// New path ->  mono/repo/dirpath
                                        subpath.push(item.name);
                                        let real_path = target_path.join(subpath.part(root_path.path.len(), subpath.path.len()));
                                        if item.mode == TreeItemMode::Tree {
                                            {
                                                let mut queue = shared_queue.lock().await;
                                                queue.push_back(subpath);
                                            }
                                            // mkdir 
                                            tokio::fs::create_dir_all(real_path).await.unwrap();
                                        } else {
                                            
                                            // TODO: fetch file and save to target path. about file fetch api, refer to test_fetch_octet_stream() test func. 
                                            fetch_and_save_file(&item.id,real_path).await.unwrap();
                                        }
                                    }
                                },
                                Err(e) => {
                                    println!("Failed to parse tree: {:?}", e);
                                },
                            }
                        },
                        Err(e) => {
                            println!("Failed to get response bytes: {:?}", e);
                        },
                    }
                } else {
                    println!("Failed to fetch tree: {}", response.status());
                }
            },
            Err(e) => {
                println!("Failed to send request: {:?}", e);
            },
        }
    }
}

#[async_recursion]
async fn worker_ro_thread(    
    root_path:GPath,
    target_path:Arc<PathBuf>,
    path:GPath,
    send_tree :Sender<(GPath,Tree)>
){
        let tree = fetch_tree(&path).await.unwrap();
        trace!("path:{}",path);
        let _ = send_tree.send((path.clone(),tree.clone())).await;
        let mut handlers = Vec::new();
        //trace!("path:{},new tree:{}",path,tree );
        for item in tree.tree_items {
            let mut subpath = path.clone();// New path ->  mono/repo/dirpath
            subpath.push(item.name);
            let real_path = target_path.join(subpath.part(root_path.path.len(), subpath.path.len()));
            if item.mode == TreeItemMode::Tree {
                {
                    let root_path = root_path.clone();
                    let _path = target_path.clone();
                    let send_tree = send_tree.clone();
                    handlers.push(
                        tokio::spawn(async move {
                            worker_ro_thread(root_path,_path,subpath,send_tree.clone()).await
                        })
                    );
                }
                // mkdir 
                tokio::fs::create_dir_all(real_path).await.unwrap();
            } else {
                
                let e = fetch_and_save_file(&item.id,real_path).await;
                println!("{:?}",e);
            }
        }
        for h in handlers{
            let _ = h.await;
        }
      
}

///
/// the tree info is store in k-v database.
///     monorepo path  -> Tree
/// 
async fn fetch_code(path:&GPath, save_path : impl AsRef<Path>){

    let target_path: Arc<PathBuf> = Arc::new(save_path.as_ref().to_path_buf());
    
    // Create the save_path directory if it doesn't exist
    tokio::fs::create_dir_all(&save_path).await.unwrap();
    let rece;
    let handle;
    {
        let (send,_rece) = tokio::sync::mpsc::channel::<(GPath,Tree)>(100);
        rece = _rece;

        let p: GPath = path.clone();
        let sc = send.clone();
        let ps=target_path.clone()  ;
        handle = tokio::spawn(async move {
            worker_ro_thread( p.clone(), ps,p ,sc).await;
        });
    }


    let storepath = save_path.as_ref().parent().unwrap().join("tree.db");
    store_trees(storepath.to_str().unwrap(), rece).await;
    
    // Clean up workers (depends on how you implement worker_thread termination)
    let _ = handle.await;
    
    print!("finish ...")
}



// async fn fetch_code_nore(path:&GPath, save_path : impl AsRef<Path>){
    
//         let queue = Arc::new(Mutex::new(VecDeque::new()));
//         let queue_clone = queue.clone();
       
//         let p = path.clone();
//         let _handle = tokio::spawn(async move {
//             // Initialize the queue with a path
//             let mut queue = queue_clone.lock().await;
//             queue.push_back( p);
//         });
//         let mut handles = vec![];
//         let target_path: Arc<PathBuf> = Arc::new(save_path.as_ref().to_path_buf());
        
//         // Create the save_path directory if it doesn't exist
//         tokio::fs::create_dir_all(&save_path).await.unwrap();
//         let rece;
//         {
//             let (s,r) = tokio::sync::mpsc::channel::<Tree>(100);
//             rece = r;
//             for i in 0..10 {
//                 let p: GPath = path.clone();
//                 let queue_clone = queue.clone();
//                 let o = target_path.clone();
//                 let ss = s.clone();
//                 let handle = tokio::spawn(async move {
//                     worker_thread(i, p, &o, queue_clone,ss).await;
//                 });
//                 handles.push(handle);
//             }
//         }

//         // Clean up workers (depends on how you implement worker_thread termination)
//         for handle in handles {
//             let _ = handle.await;
//         }
//         let storepath = save_path.as_ref().parent().unwrap().join("tree.db");
//         store::store_trees(storepath.to_str().unwrap(), rece).await;
//         // Check if the queue has been populated
//         let queue = queue.lock().await;
//         assert!(queue.len() == 0);
     
// }


async fn fetch_and_save_file(url: &SHA1, save_path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let file_blob_endpoint = scorpio_config::file_blob_endpoint();
    let url = format!("{}/{}",file_blob_endpoint,url);
    // Send GET request
    let response = client.get(url).send().await?;
    
    // Ensure that the response status is successful
    if response.status().is_success() {
        // Get the binary data from the response body
        let content = response.bytes().await?;
        
        // Store the content in a Vec<u8>
        let data: Vec<u8> = content.to_vec();
        
        // Save the data to a file
        tokio::fs::write(save_path, data).await?;
        
    } else {
        eprintln!("Request failed with status: {}", response.status());
    }
    
    Ok(())
}

#[allow(unused)]
pub async fn fetch_tree(path: &GPath) -> Result<Tree, Box<dyn std::error::Error>> {
    let url = format!("{}{}", BASE_URL, path);
    let response = reqwest::get(&url).await?;
    
    if response.status().is_success() {
        let bytes = response.bytes().await?;
        let tree = Tree::try_from(&bytes[..])?;
        Ok(tree)
    } else {
        Err(format!("Failed to fetch tree: {}", response.status()).into())
    }
}
          

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use std::error::Error;
    use mercury::internal::object::tree::Tree;
    use std::fs::File;
    #[tokio::test]
    async fn test_fetch_octet_stream() -> Result<(), Box<dyn Error>> {
        // Create an HTTP client
        let client = Client::new();
        
        // Use the URL from environment variables or local test URL
        let url = "http://localhost:8000/api/v1/file/tree?path=/third-part/mega";
        
        // Send GET request
        let response = client.get(url).send().await?;
        
        // Ensure that the response status is successful
        if response.status().is_success() {
            // Get the binary data from the response body
            let content = response.bytes().await?;
            
            // Store the content in a Vec<u8>
            let data: Vec<u8> = content.to_vec();
            let tree = Tree::try_from(&data[..]).unwrap();
            // Print the data length for testing assertions
            // println!("Received {} bytes of data", data.len());
            
            // // You can add more assertions or validation logic here
            // assert!(!data.is_empty(), "Data should not be empty");

            println!("{}",tree);
            // You can also validate the specific content of the data
            // assert_eq!(data, expected_data); // You need to define expected_data
            
        } else {
            eprintln!("Request failed with status: {}", response.status());
            return Err(format!("Request failed with status: {}", response.status()).into());
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_octet_file() {
        // Create an HTTP client
        let client = Client::new();

        // Use the URL from environment variables or local test URL
        let url = "http://localhost:8000/api/v1/file/blob/841b6fe34540e866e1f458d77b1bd03d3cb0e782";
        // Send a GET request
        let response = client.get(url).send().await.unwrap();

        // Ensure that the response status is successful
        if response.status().is_success() {
            // Get the binary data from the response body
            let content = response.bytes().await.unwrap();

            // Store the content in a Vec<u8>
            let data: Vec<u8> = content.to_vec();
            use std::io::prelude::*;
            // Save the data to a file
            let mut file = File::create("output.txt").unwrap();
            file.write_all(&data).unwrap();

            // Print the path to the saved file
            println!("Data saved to output.txt");
        } else {
            eprintln!("Request failed with status: {}", response.status());
        }
    }

}