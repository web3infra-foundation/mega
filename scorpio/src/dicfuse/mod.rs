mod abi;
mod async_io;
mod content_store;
pub mod manager;
mod size_store;
pub mod store;
mod tree_store;

pub use manager::DicfuseManager;

use crate::manager::fetch::fetch_tree;
use crate::util::config;
use std::{
    ffi::{OsStr, OsString},
    sync::Arc,
};

/// Compute the backing store directory for a given base path.
///
/// - Global (root) view uses the configured `store_root` directly.
/// - Subdirectory stores are under "{store_root}/dicfuse/{sha256(base_path)[:16]}".
pub(crate) fn compute_store_dir_for_base_path_with_store_root(
    store_root: &str,
    base_path: &str,
) -> String {
    let normalized = if base_path.is_empty() || base_path == "/" {
        "/".to_string()
    } else {
        base_path.trim_end_matches('/').to_string()
    };

    if normalized == "/" {
        store_root.to_string()
    } else {
        let digest = ring::digest::digest(&ring::digest::SHA256, normalized.as_bytes());
        let hex = hex::encode(digest.as_ref());
        format!("{}/dicfuse/{}", store_root, &hex[..16])
    }
}

use async_trait::async_trait;
use git_internal::internal::object::tree::TreeItemMode;
use libfuse_fs::unionfs::Inode;
use libfuse_fs::{context::OperationContext, unionfs::layer::Layer};
use reqwest::Client;
use rfuse3::raw::reply::{ReplyCreated, ReplyEntry};
use rfuse3::Result;
use store::DictionaryStore;
use tree_store::StorageItem;

pub struct Dicfuse {
    readable: bool,
    pub store: Arc<DictionaryStore>,
}
unsafe impl Sync for Dicfuse {}
unsafe impl Send for Dicfuse {}

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

    /// Retrieve metadata with optional ID mapping control.
    ///
    /// For Dicfuse (a virtual read-only layer), we ignore the `mapping` flag and
    /// construct a synthetic `stat64` from our in-memory `StorageItem`, similar
    /// to the old `do_getattr_helper` behavior in earlier libfuse-fs versions.
    async fn getattr_with_mapping(
        &self,
        inode: Inode,
        _handle: Option<u64>,
        _mapping: bool,
    ) -> std::io::Result<(libc::stat64, std::time::Duration)> {
        // Resolve inode -> StorageItem to derive type/size.
        let item = self
            .store
            .get_inode(inode)
            .await
            .map_err(|_| std::io::Error::from_raw_os_error(libc::ENOENT))?;

        // Use existing ReplyEntry metadata to stay consistent with other Dicfuse paths.
        let attr = item.get_stat().attr;

        let size: i64 = if item.is_dir() {
            0
        } else {
            self.store
                .get_or_fetch_file_size(inode, &item.hash)
                .await
                .min(i64::MAX as u64) as i64
        };

        let type_bits: libc::mode_t = match attr.kind {
            rfuse3::FileType::Directory => libc::S_IFDIR,
            rfuse3::FileType::Symlink => libc::S_IFLNK,
            _ => libc::S_IFREG,
        };

        let perm: libc::mode_t = if item.is_dir() {
            attr.perm as libc::mode_t
        } else if self.store.is_executable(inode) {
            0o755
        } else {
            0o644
        };
        let mode: libc::mode_t = type_bits | perm;
        let nlink = if attr.nlink > 0 {
            attr.nlink
        } else if item.is_dir() {
            2
        } else {
            1
        };

        // Construct stat64 structure using zeroed() for platform-specific padding fields.
        let mut stat: libc::stat64 = unsafe { std::mem::zeroed() };
        stat.st_dev = 0;
        stat.st_ino = inode;
        stat.st_nlink = nlink as _;
        stat.st_mode = mode;
        stat.st_uid = attr.uid;
        stat.st_gid = attr.gid;
        stat.st_rdev = 0;
        stat.st_size = size;
        stat.st_blksize = 4096;
        stat.st_blocks = (size + 511) / 512; // Round up to 512-byte blocks
        stat.st_atime = attr.atime.sec;
        stat.st_atime_nsec = attr.atime.nsec.into();
        stat.st_mtime = attr.mtime.sec;
        stat.st_mtime_nsec = attr.mtime.nsec.into();
        stat.st_ctime = attr.ctime.sec;
        stat.st_ctime_nsec = attr.ctime.nsec.into();

        // TTL of 2 seconds, consistent with other Dicfuse operations.
        Ok((stat, std::time::Duration::from_secs(2)))
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

    /// Create a new Dicfuse instance with a base path and an explicit store path.
    ///
    /// This is useful for Antares build scenarios where multiple mounts may coexist and we want
    /// to isolate on-disk caches to avoid sled DB lock conflicts.
    pub async fn new_with_base_path_and_store_path(base_path: &str, store_path: &str) -> Self {
        Self {
            readable: config::dicfuse_readable(),
            store: DictionaryStore::new_with_base_path_and_store_path(base_path, store_path)
                .await
                .into(),
        }
    }

    /// Create a new Dicfuse instance with a base path for subdirectory mounting.
    ///
    /// When `base_path` is set (e.g., "/third-party/mega"), the filesystem will:
    /// - Only expose content under the specified path
    /// - Remap paths so the base_path becomes the root "/"
    ///
    /// This is useful for Antares build scenarios where only a specific
    /// subdirectory of the monorepo is needed for a build task.
    ///
    /// # Arguments
    /// * `base_path` - The subdirectory path to use as root (e.g., "/third-party/mega")
    ///
    /// # Example
    /// ```ignore
    /// let dicfuse = Dicfuse::new_with_base_path("/third-party/mega").await;
    /// // Accessing "/" in this dicfuse actually accesses "/third-party/mega" in the monorepo
    /// ```
    pub async fn new_with_base_path(base_path: &str) -> Self {
        Self {
            readable: config::dicfuse_readable(),
            store: DictionaryStore::new_with_base_path(base_path).await.into(),
        }
    }

    /// Get the base path of this Dicfuse instance.
    ///
    /// Returns an empty string if no base path is set (full monorepo access).
    pub fn base_path(&self) -> &str {
        self.store.base_path()
    }

    pub async fn get_stat(&self, item: StorageItem) -> ReplyEntry {
        let mut e = item.get_stat();
        if item.is_dir() {
            e.attr.size = 0;
            return e;
        }

        let size = self
            .store
            .file_size_for_stat(item.get_inode(), &item.hash)
            .await;
        e.attr.size = size;
        e
    }

    /// Fast stat helper for hot paths (e.g., readdirplus): avoids any network IO.
    /// Size will be taken from persisted size metadata if present; otherwise it may be 0 until
    /// a later getattr/open triggers size discovery.
    pub async fn get_stat_fast(&self, item: StorageItem) -> ReplyEntry {
        let mut e = item.get_stat();
        if item.is_dir() {
            e.attr.size = 0;
            return e;
        }
        e.attr.size = self.store.get_persisted_size(item.get_inode()).unwrap_or(0);
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
                if i.mode == TreeItemMode::BlobExecutable {
                    self.store.set_executable(it_temp.get_inode(), true);
                }
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
        let gpath = match self.store.find_path(parent_item.get_inode()).await {
            Some(p) => p,
            None => {
                tracing::warn!(
                    "load_files: find_path missing for inode {}",
                    parent_item.get_inode()
                );
                return;
            }
        };
        let tree = match fetch_tree(&gpath).await {
            Ok(t) => t,
            Err(err) => {
                tracing::warn!(
                    "load_files: fetch_tree failed for path {}: {err}",
                    gpath.to_string()
                );
                return;
            }
        };
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
            let response = match client.get(url).send().await {
                Ok(resp) => resp,
                Err(err) => {
                    tracing::warn!("load_files: request failed for {}: {err}", i.id);
                    continue;
                }
            };

            // Ensure that the response status is successful
            if response.status().is_success() {
                // Get the binary data from the response body
                let content = match response.bytes().await {
                    Ok(b) => b,
                    Err(err) => {
                        tracing::warn!("load_files: read body failed for {}: {err}", i.id);
                        continue;
                    }
                };

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
                let Some(hit_inodes) = hit_inodes else {
                    tracing::warn!(
                        "load_files: inode not found for name {} in parent {}",
                        i.name,
                        gpath.to_string()
                    );
                    continue;
                };

                // Look up the buff, find Loaded file.
                if is_first {
                    if self.store.file_exists(hit_inodes) {
                        // if the file is already exists, no need to load again.
                        break;
                    }
                    self.store.save_file(hit_inodes, data);
                    if i.mode == TreeItemMode::BlobExecutable {
                        self.store.set_executable(hit_inodes, true);
                    }
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
    use std::path::PathBuf;

    use tokio::signal;

    use crate::dicfuse::Dicfuse;
    use libfuse_fs::unionfs::layer::Layer;

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

    #[tokio::test]
    async fn test_getattr_with_mapping_preserves_mode_and_size() {
        let base = PathBuf::from("/tmp/dicfuse_attr_test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let dic = Dicfuse::new_with_store_path(base.to_str().unwrap()).await;

        // Insert root and one file; mark file executable and with known content length.
        dic.store.insert_mock_item(1, 0, "", true).await;
        dic.store.insert_mock_item(2, 1, "file", false).await;
        dic.store.save_file(2, b"abc".to_vec());
        dic.store.set_executable(2, true);

        let (file_stat, _) = dic.getattr_with_mapping(2, None, false).await.unwrap();
        assert_eq!(file_stat.st_mode & libc::S_IFMT, libc::S_IFREG);
        assert_eq!(file_stat.st_mode & 0o777, 0o755);
        assert_eq!(file_stat.st_size, 3);

        let (dir_stat, _) = dic.getattr_with_mapping(1, None, false).await.unwrap();
        assert_eq!(dir_stat.st_mode & libc::S_IFMT, libc::S_IFDIR);
        assert_eq!(dir_stat.st_mode & 0o777, 0o755);
        assert_eq!(dir_stat.st_nlink, 2);

        let _ = std::fs::remove_dir_all(&base);
    }
}
