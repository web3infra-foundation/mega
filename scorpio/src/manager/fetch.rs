
use std::path::Path;
use std::{collections::VecDeque, sync::Arc, time::Duration};
use mercury::hash::SHA1;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use reqwest::Client;
use tokio::sync::Mutex;
use tokio::time;


use crate::util::GPath;
const BASE_URL : &str = "http://localhost:8000/api/v1/file/tree?path=/";
#[allow(unused)]
#[allow(clippy::blocks_in_conditions)]
async fn worker_thread(
    id:u32,
    root_path:GPath,
    target_path:&Path,
    shared_queue: Arc<Mutex<VecDeque<GPath>>>
) {
    let client = Client::new();
    let mut interval = time::interval(Duration::from_millis(50)); // 设定检查间隔时间
    let timeout_duration = Duration::from_millis(100);
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

async fn fetch_and_save_file(url: &SHA1, save_path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
     let url = format!("http://localhost:8000/api/v1/file/blob/{}",url.to_plain_str());
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
async fn fetch_tree(path: GPath) -> Result<Tree, Box<dyn std::error::Error>> {
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
mod tests2 {
    use reqwest::Client;
    use tokio::sync::Mutex;
    use std::{collections::VecDeque, error::Error, path::Path, sync::Arc};
    use mercury::internal::object::tree::Tree;
    use crate::{manager::fetch::worker_thread, util::GPath};
    use std::fs::File;
    #[tokio::test]
    async fn test_fetch_octet_stream() -> Result<(), Box<dyn Error>> {
        // Create an HTTP client
        let client = Client::new();
        
        // Use the URL from environment variables or local test URL
        let url = "http://localhost:8000/api/v1/file/tree?path=/third-part/";
        
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
    async fn test_fetch_tree( ){
        //env_logger::builder().filter_level(log::LevelFilter::Trace).init();
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let queue_clone = queue.clone();
        let mut path = GPath::new();
        path.push(String::from("third-part"));
        let p = path.clone();
        let _handle = tokio::spawn(async move {
            // Initialize the queue with a path
            let mut queue = queue_clone.lock().await;
            queue.push_back( p);
        });
        let mut handles = vec![];
        
        let target_path = Arc::new(Path::new("/home/luxian/megatest/hash1"));
        for i in 0..10 {
            let p: GPath = path.clone();
            let queue_clone = queue.clone();
            let o = target_path.clone();
            let handle = tokio::spawn(worker_thread(i,p, &o, queue_clone));
            handles.push(handle);
        }
                    // Clean up workers (depends on how you implement worker_thread termination)
        for handle in handles {
            let _ = handle.await;
        }
        
        // Check if the queue has been populated
        let queue = queue.lock().await;
        assert!(queue.len() == 0);

    }
    #[tokio::test]
    async fn test_fetch_octet_file() {
        // Create an HTTP client
        let client = Client::new();

        // Use the URL from environment variables or local test URL
        let url = "http://localhost:8000/api/v1/file/blob/d12d12579799a658b29808fe695abd919a033ac9";

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