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
    time::Duration,
};

use async_trait::async_trait;
use git_internal::internal::object::tree::TreeItemMode;
use libfuse_fs::unionfs::Inode;
use libfuse_fs::{context::OperationContext, unionfs::layer::Layer};
use reqwest::Client;
use rfuse3::raw::reply::{FileAttr, ReplyCreated, ReplyEntry};
use rfuse3::FileType as FuseFileType;
use rfuse3::Result;
use store::DictionaryStore;
use tree_store::StorageItem;

pub struct Dicfuse {
    readable: bool,
    pub store: Arc<DictionaryStore>,
}
unsafe impl Sync for Dicfuse {}
unsafe impl Send for Dicfuse {}

/// Convert FileAttr to libc::stat64 for OverlayFs copy-up operations.
fn fileattr_to_stat64(attr: &FileAttr) -> libc::stat64 {
    unsafe {
        let mut st: libc::stat64 = std::mem::zeroed();
        st.st_ino = attr.ino as libc::ino64_t;
        st.st_size = attr.size as libc::off_t;
        st.st_blocks = attr.blocks as libc::blkcnt64_t;
        st.st_uid = attr.uid as libc::uid_t;
        st.st_gid = attr.gid as libc::gid_t;
        // File type bits (S_IF*)
        let type_bits: libc::mode_t = match attr.kind {
            FuseFileType::NamedPipe => libc::S_IFIFO,
            FuseFileType::CharDevice => libc::S_IFCHR,
            FuseFileType::BlockDevice => libc::S_IFBLK,
            FuseFileType::Directory => libc::S_IFDIR,
            FuseFileType::RegularFile => libc::S_IFREG,
            FuseFileType::Symlink => libc::S_IFLNK,
            FuseFileType::Socket => libc::S_IFSOCK,
        };

        // Permission bits
        let perm_bits = attr.perm as libc::mode_t;
        st.st_mode = type_bits | perm_bits;
        st.st_rdev = attr.rdev as libc::dev_t;
        st.st_blksize = attr.blksize as libc::blksize_t;
        st.st_nlink = attr.nlink as libc::nlink_t;
        st
    }
}

#[async_trait]
impl Layer for Dicfuse {
    fn root_inode(&self) -> Inode {
        1
    }

    /// Create a file in the layer (not supported for read-only Dicfuse).
    /// This is called by OverlayFs during copy-up operations.
    async fn create_with_context(
        &self,
        _ctx: OperationContext,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _flags: u32,
    ) -> Result<ReplyCreated> {
        // Dicfuse is a read-only layer, does not support file creation
        tracing::warn!(
            "[{}:{}] create_with_context not supported on Dicfuse (read-only)",
            file!(),
            line!()
        );
        Err(std::io::Error::from_raw_os_error(libc::EROFS).into())
    }

    /// Create a directory in the layer (not supported for read-only Dicfuse).
    /// This is called by OverlayFs during copy-up operations.
    async fn mkdir_with_context(
        &self,
        _ctx: OperationContext,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
    ) -> Result<ReplyEntry> {
        // Dicfuse is a read-only layer, does not support directory creation
        tracing::warn!(
            "[{}:{}] mkdir_with_context not supported on Dicfuse (read-only)",
            file!(),
            line!()
        );
        Err(std::io::Error::from_raw_os_error(libc::EROFS).into())
    }

    /// Create a symlink in the layer (not supported for read-only Dicfuse).
    /// This is called by OverlayFs during copy-up operations.
    async fn symlink_with_context(
        &self,
        _ctx: OperationContext,
        _parent: Inode,
        _name: &OsStr,
        _link: &OsStr,
    ) -> Result<ReplyEntry> {
        // Dicfuse is a read-only layer, does not support symlink creation
        tracing::warn!(
            "[{}:{}] symlink_with_context not supported on Dicfuse (read-only)",
            file!(),
            line!()
        );
        Err(std::io::Error::from_raw_os_error(libc::EROFS).into())
    }

    /// Retrieve host-side metadata bypassing ID mapping.
    /// This is used internally by OverlayFs copy-up operations to get raw stat information.
    async fn do_getattr_helper(
        &self,
        inode: Inode,
        _handle: Option<u64>,
    ) -> std::io::Result<(libc::stat64, Duration)> {
        // Reuse Dicfuse's existing stat logic
        let item = self.store.get_inode(inode).await?;
        let entry = self.get_stat(item).await;
        let st = fileattr_to_stat64(&entry.attr);
        Ok((st, entry.ttl))
    }
}

#[allow(unused)]
impl Dicfuse {
    pub async fn new() -> Self {
        Self {
            readable: config::dicfuse_readable(),
            store: DictionaryStore::new().await.into(), // Assuming DictionaryStore has a new() method
        }
    }

    pub async fn new_with_store_path(store_path: &str) -> Self {
        Self {
            readable: config::dicfuse_readable(),
            store: DictionaryStore::new_with_store_path(store_path)
                .await
                .into(),
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
    #[ignore = "manual test requiring root privileges for FUSE mount"]
    async fn test_mount_dic() {
        // Use environment variable or default to temp directory
        let mount_path =
            std::env::var("DIC_MOUNT_PATH").unwrap_or_else(|_| "/tmp/test_dic_mount".to_string());

        // Create mount directory if it doesn't exist
        std::fs::create_dir_all(&mount_path).expect("Failed to create mount directory");

        let fs = Dicfuse::new().await;
        let mountpoint = OsStr::new(&mount_path);
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
