mod abi;
mod async_io;
mod content_store;
pub mod store;
mod tree_store;
use crate::manager::fetch::fetch_tree;
use crate::util::config;
use std::{
    ffi::{OsStr, OsString},
    sync::Arc,
};

use git_internal::internal::object::tree::TreeItemMode;
use reqwest::Client;
use rfuse3::raw::reply::ReplyEntry;
use store::DictionaryStore;
use tree_store::StorageItem;

pub struct Dicfuse {
    readable: bool,
    pub store: Arc<DictionaryStore>,
}
#[allow(unused)]
impl Dicfuse {
    pub async fn new() -> Self {
        Self {
            readable: config::dicfuse_readable(),
            store: DictionaryStore::new().await.into(), // Assuming DictionaryStore has a new() method
        }
    }
    pub async fn get_stat(&self, item: StorageItem) -> ReplyEntry {
        let mut e = item.get_stat();
        e.attr.size = self.store.get_file_len(item.get_inode());
        e
    }
    async fn load_one_file(&self, parent: u64, name: &OsStr) -> std::io::Result<()> {
        if !self.readable {
            return Ok(());
        }

        let mut parent_item = self.store.find_path(parent).await.unwrap();
        let tree = fetch_tree(&parent_item).await.unwrap();

        let file_blob_endpoint = config::file_blob_endpoint();

        let client = Client::new();
        for i in tree.tree_items {
            let name_os = OsString::from(&i.name);
            if name_os != name {
                continue;
            } else if i.mode != TreeItemMode::Blob && i.mode != TreeItemMode::BlobExecutable {
                return Ok(());
            }

            let url = format!("{}/{}", file_blob_endpoint, i.id);
            // Send GET request
            let response = client.get(url).send().await.unwrap(); //todo error

            // Ensure that the response status is successful
            if response.status().is_success() {
                // Get the binary data from the response body
                let content = response.bytes().await.unwrap(); //TODO error

                // Store the content in a Vec<u8>
                let data: Vec<u8> = content.to_vec();
                //let child_osstr = OsStr::new(&i.name);
                parent_item.push(i.name.clone());

                let it_temp = self.store.get_by_path(&parent_item.to_string()).await?;
                self.store.save_file(it_temp.get_inode(), data);
            } else {
                eprintln!("Request failed with status: {}", response.status());
            }
            break;
        }
        Ok(())
    }
    pub async fn load_files(&self, parent_item: StorageItem, items: &Vec<StorageItem>) {
        if !self.readable {
            return;
        }
        if self.store.file_exists(parent_item.get_inode()) {
            return;
        }
        let gpath = self.store.find_path(parent_item.get_inode()).await.unwrap();
        let tree = fetch_tree(&gpath).await.unwrap();
        let mut is_first = true;
        let client = Client::new();
        let file_blob_endpoint = config::file_blob_endpoint();
        for i in tree.tree_items {
            //TODO & POS_BUG: how to deal with the link?
            if i.mode != TreeItemMode::Blob && i.mode != TreeItemMode::BlobExecutable {
                continue;
            }
            let url = format!("{}/{}", file_blob_endpoint, i.id);
            // Send GET request
            let response = client.get(url).send().await.unwrap(); //todo error

            // Ensure that the response status is successful
            if response.status().is_success() {
                // Get the binary data from the response body
                let content = response.bytes().await.unwrap(); //TODO error

                // Store the content in a Vec<u8>
                let data: Vec<u8> = content.to_vec();

                // Get the hit inodes.
                let mut hit_inodes: Option<u64> = None;
                for it in items {
                    if it.name.eq(&i.name) {
                        hit_inodes = Some(it.get_inode());
                        break;
                    }
                }
                assert!(hit_inodes.is_some()); // must find an inode from children.
                let hit_inodes = hit_inodes.unwrap();

                // Look up the buff, find Loaded file.
                if is_first {
                    if self.store.file_exists(hit_inodes) {
                        // if the file is already exists, no need to load again.
                        break;
                    }
                    self.store.save_file(hit_inodes, data);
                    is_first = false;
                }
            } else {
                eprintln!("Request failed with status: {}", response.status());
            }
        }
        self.store.save_file(parent_item.get_inode(), Vec::new());
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use tokio::signal;

    use crate::dicfuse::Dicfuse;

    #[tokio::test]
    #[ignore]
    async fn test_mount_dic() {
        let fs = Dicfuse::new().await;
        let mountpoint = OsStr::new("/home/luxian/dic");
        let mut mount_handle = crate::server::mount_filesystem(fs, mountpoint).await;
        let handle = &mut mount_handle;
        tokio::select! {
            res = handle => res.unwrap(),
            _ = signal::ctrl_c() => {
                mount_handle.unmount().await.unwrap()
            }
        }
    }
}
