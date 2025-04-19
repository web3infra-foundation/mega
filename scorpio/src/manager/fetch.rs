use axum::async_trait;
use ceres::model::git::LatestCommitInfo;
use mercury::hash::SHA1;
use mercury::internal::object::tree::{Tree, TreeItemMode};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{collections::VecDeque, sync::Arc, time::Duration};

use async_recursion::async_recursion;
use mercury::internal::object::{
    commit::Commit,
    signature::{Signature, SignatureType},
};
use std::io::Write;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::time;

use crate::manager::store::store_trees;
use crate::scolfs;
use crate::util::config;
use crate::util::GPath;

use super::{ScorpioManager, WorkDir};

#[allow(unused)]
#[async_trait]
pub trait CheckHash {
    async fn check(&mut self);

    async fn fetch<P: AsRef<Path> + std::marker::Send>(
        &mut self,
        inode: u64,
        monopath: P,
    ) -> WorkDir;
}

#[async_trait]
impl CheckHash for ScorpioManager {
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
                let store_path = config::store_path();
                let _lower = PathBuf::from(store_path).join(&work.hash).join("lower");
                handlers.push(tokio::spawn(async move { fetch_code(&p, _lower).await }));
            }
        }
        // if have new config path , finish all handlers and write back the config file
        if !handlers.is_empty() {
            for handle in handlers {
                let _ = handle.await;
            }
            //Get config file path from scorpio_config.rs
            let config_file = config::config_file();
            let _ = self.to_toml(config_file);
        }
    }

    async fn fetch<P: AsRef<Path> + std::marker::Send>(
        &mut self,
        inode: u64,
        monopath: P,
    ) -> WorkDir {
        let path = monopath.as_ref().to_str().unwrap().to_string();
        let p = GPath::from(path);
        // Get the tree and its hash value, for name dictionary .
        let tree = fetch_tree(&p).await.unwrap();
        let workdir = WorkDir {
            path: p.to_string(),
            node: inode,
            hash: tree.id.to_string(),
        };
        //work.hash = tree.id.to_string();
        // the lower path is store file path for remote code version .
        let store_path = config::store_path();
        let _lower = PathBuf::from(store_path).join(&workdir.hash).join("lower");
        fetch_code(&p, _lower).await.unwrap();
        self.works.push(workdir.clone());
        let config_file = config::config_file();
        let _ = self.to_toml(config_file);

        workdir
    }
}

/// The core function of fetch operation
pub async fn fetch<P: AsRef<Path>>(
    manager: &mut ScorpioManager,
    inode: u64,
    monopath: P,
) -> std::io::Result<WorkDir> {
    let path = monopath.as_ref().to_str().unwrap().to_string();
    let p = GPath::from(path);
    // Get the tree and its hash value, for name dictionary .
    let tree = fetch_tree(&p).await.unwrap();
    let workdir = WorkDir {
        path: p.to_string(),
        node: inode,
        hash: tree.id.to_string(),
    };
    //work.hash = tree.id.to_string();
    // the lower path is store file path for remote code version .
    let store_path = config::store_path();
    let work_path = PathBuf::from(store_path).join(&workdir.hash);
    let _lower = work_path.join("lower");
    fetch_code(&p, _lower).await?;
    manager.works.push(workdir.clone());
    let config_file = config::config_file();
    let _ = manager.to_toml(config_file);

    // Get the commit information of the previous version and
    // write it into the commit file.
    set_parent_commit(&work_path).await?;

    Ok(workdir)
}

#[allow(unused)]
#[allow(clippy::blocks_in_conditions)]
async fn worker_thread(
    id: u32,
    root_path: GPath,
    target_path: &Path,
    shared_queue: Arc<Mutex<VecDeque<GPath>>>,
    send_tree: Sender<Tree>,
) {
    let client = Client::new();
    //let mut interval = time::interval(Duration::from_millis(50));
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
        let url = format!("{}{}", config::tree_file_endpoint(), path);
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            match Tree::try_from(&bytes[..]) {
                                Ok(tree) => {
                                    trace!("ID:{},path:{}", id, path);
                                    send_tree.send(tree.clone()).await;
                                    //trace!("path:{},new tree:{}",path,tree );
                                    for item in tree.tree_items {
                                        let mut subpath = path.clone(); // New path ->  mono/repo/dirpath
                                        subpath.push(item.name);
                                        let real_path = target_path.join(
                                            subpath.part(root_path.path.len(), subpath.path.len()),
                                        );
                                        if item.mode == TreeItemMode::Tree {
                                            {
                                                let mut queue = shared_queue.lock().await;
                                                queue.push_back(subpath);
                                            }
                                            // mkdir
                                            tokio::fs::create_dir_all(real_path).await.unwrap();
                                        } else {
                                            // TODO: fetch file and save to target path. about file fetch api, refer to test_fetch_octet_stream() test func.
                                            fetch_and_save_file(&item.id, real_path).await.unwrap();
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to parse tree: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Failed to get response bytes: {:?}", e);
                        }
                    }
                } else {
                    println!("Failed to fetch tree: {}", response.status());
                }
            }
            Err(e) => {
                println!("Failed to send request: {:?}", e);
            }
        }
    }
}

/// Network operations, recursively read the main Tree from the pipe, and download Blobs objects
#[async_recursion]
async fn worker_ro_thread(
    root_path: GPath,
    target_path: Arc<PathBuf>,
    path: GPath,
    send_tree: Sender<(GPath, Tree)>,
) {
    let tree = fetch_tree(&path).await.unwrap();
    trace!("path:{}", path);
    let _ = send_tree.send((path.clone(), tree.clone())).await;
    let mut handlers = Vec::new();
    //trace!("path:{},new tree:{}",path,tree );
    for item in tree.tree_items {
        let mut subpath = path.clone(); // New path ->  mono/repo/dirpath
        subpath.push(item.name);
        let real_path = target_path.join(subpath.part(root_path.path.len(), subpath.path.len()));
        if item.mode == TreeItemMode::Tree {
            {
                let root_path = root_path.clone();
                let _path = target_path.clone();
                let send_tree = send_tree.clone();
                handlers.push(tokio::spawn(async move {
                    worker_ro_thread(root_path, _path, subpath, send_tree.clone()).await
                }));
            }
            // mkdir
            tokio::fs::create_dir_all(real_path).await.unwrap();
        } else {
            let e = fetch_and_save_file(&item.id, real_path).await;
            println!("{:?}", e);
        }
    }
    for h in handlers {
        let _ = h.await;
    }
}

///
/// the tree info is store in k-v database.
///     monorepo path  -> Tree
///
/// Download remote data to local and store it in Overlay format
async fn fetch_code(path: &GPath, save_path: impl AsRef<Path>) -> std::io::Result<()> {
    let target_path: Arc<PathBuf> = Arc::new(save_path.as_ref().to_path_buf());

    // Create the save_path directory if it doesn't exist
    tokio::fs::create_dir_all(&save_path).await?;
    let rece;
    let handle;
    {
        // Set up a pipeline with a capacity of 100
        let (send, _rece) = tokio::sync::mpsc::channel::<(GPath, Tree)>(100);
        rece = _rece;

        let p: GPath = path.clone();
        let sc = send.clone();
        let ps = target_path.clone();
        // Use child threads to operate mounted directory
        handle = tokio::spawn(async move {
            worker_ro_thread(p.clone(), ps, p, sc).await;
        });
    }

    let storepath = save_path.as_ref().parent().unwrap().join("tree.db");
    store_trees(storepath.to_str().unwrap(), rece).await?;

    // Clean up workers (depends on how you implement worker_thread termination)
    let _ = handle.await;

    //get lfs file
    scolfs::lfs::lfs_restore(&path.to_string(), save_path.as_ref().to_str().unwrap())
        .await
        .unwrap();

    print!("finish code for {}...", path);

    Ok(())
}

/// Get the previous version of the Commit information from the remote API,
/// convert it into a Commit structure, and write it into the commit file.
async fn set_parent_commit(work_path: &Path) -> std::io::Result<()> {
    let parent_commit = match fetch_parent_commit().await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Failed to fetch parent commit info: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to fetch parent commit info"));
        }
    };

    let path = work_path.join("commit");
    let mut commit_file = std::fs::File::create(&path)?;
    commit_file.write_all(parent_commit.to_string().as_bytes())?;

    Ok(())
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

/// Network operations, extracting Blobs objects from HTTP byte streams and storing them
async fn fetch_and_save_file(
    url: &SHA1,
    save_path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let file_blob_endpoint = config::file_blob_endpoint();
    let url = format!("{}/{}", file_blob_endpoint, url);
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

/// Network operations, extracting Tree objects from HTTP byte streams
#[allow(unused)]
pub async fn fetch_tree(path: &GPath) -> Result<Tree, Box<dyn std::error::Error>> {
    let url = format!("{}{}", config::tree_file_endpoint(), path);
    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let bytes = response.bytes().await?;
        let tree = Tree::try_from(&bytes[..])?;
        Ok(tree)
    } else {
        Err(format!("Failed to fetch tree: {}", response.status()).into())
    }
}

/// Network operations, extracting parent commit Hash from HTTP byte streams
pub async fn fetch_parent_commit() -> Result<Commit, Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/latest-commit", config::base_url());
    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let parent_info = response.json::<LatestCommitInfo>().await?;
        let author_sign = Signature::new(
            SignatureType::Author,
            parent_info.author.display_name.clone(),
            parent_info.author.avatar_url.clone(),
        );
        let committer_sign = Signature::new(
            SignatureType::Committer,
            parent_info.committer.display_name.clone(),
            parent_info.committer.avatar_url.clone(),
        );
        Ok(Commit::new(
            author_sign,
            committer_sign,
            SHA1::from_str(&parent_info.oid)?,
            Vec::new(),
            &parent_info.short_message,
        ))
    } else {
        Err(format!("Failed to fetch tree: {}", response.status()).into())
    }
}

#[cfg(test)]
mod tests {
    use mercury::internal::object::tree::Tree;
    use reqwest::Client;
    use std::error::Error;
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

            println!("{}", tree);
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
