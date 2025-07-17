use inode_alloc::InodeAlloc;
use libfuse_fs::{
    overlayfs::{config, OverlayFs},
    passthrough::new_passthroughfs_layer,
};
use rfuse3::raw::{Filesystem, Request};
use tokio::sync::Mutex;

use crate::util::config as sconfig;
use crate::{dicfuse::Dicfuse, manager::ScorpioManager};
use std::{
    collections::HashMap,
    io::Error,
    path::{Path, PathBuf},
    sync::Arc,
};

mod async_io;
mod inode_alloc;

/// A struct representing the MegaFuse system, which handles the creation
/// and management of overlay filesystems (OverlayFs). This includes
/// mounting and unmounting operations, as well as inode allocation and
/// other filesystem-related functionalities.
///
/// The `MegaFuse` struct contains the following:
/// - `dic`: A reference-counted pointer to the Dicfuse system for dictionary-based operations.
/// - `overlayfs`: A Mutex-wrapped `HashMap` storing the overlay filesystems for each inode.
/// - `inodes_alloc`: A struct responsible for allocating inodes.
#[allow(unused)]
#[derive(Clone)]
pub struct MegaFuse {
    pub dic: Arc<Dicfuse>,
    overlayfs: Arc<Mutex<HashMap<u64, Arc<OverlayFs>>>>, // Inode -> overlayyfs
    inodes_alloc: InodeAlloc,
}

#[allow(unused)]
impl MegaFuse {
    /// Creates a new instance of `MegaFuse` asynchronously.
    ///
    /// This function initializes the `dic`, `overlayfs`, and `inodes_alloc` fields
    /// of the `MegaFuse` struct. It is used for creating a base `MegaFuse` object
    /// before performing additional setup or mounting operations.
    ///
    /// # Returns
    /// A new `MegaFuse` instance.
    pub async fn new() -> Self {
        Self {
            dic: Arc::new(Dicfuse::new().await),
            overlayfs: Arc::new(Mutex::new(HashMap::new())),
            inodes_alloc: InodeAlloc::new(),
        }
    }
    /// Creates a new instance of `MegaFuse` from a given manager asynchronously.
    ///
    /// This function creates a new `MegaFuse` instance and then performs mount operations
    /// for directories based on the provided `ScorpioManager`. It mounts the user's work
    /// directories by using information from the manager and sets up the necessary overlay filesystems.
    ///
    /// # Parameters
    /// - `manager`: A reference to a `ScorpioManager` instance that holds the store path and works to mount.
    ///
    /// # Returns
    /// A new `MegaFuse` instance with mounted overlay filesystems based on the manager's configuration.
    pub async fn new_from_manager(manager: &ScorpioManager) -> MegaFuse {
        let megafuse = MegaFuse::new().await;
        let store_path = sconfig::store_path();

        // mount user works.
        for dir in &manager.works {
            let _lower = PathBuf::from(store_path).join(&dir.hash);
            megafuse
                .overlay_mount(dir.node, &_lower, false)
                .await
                .unwrap();
        }

        megafuse
    }

    /// Mounts an overlay filesystem at a specified path asynchronously.
    ///
    /// This function sets up a layered overlay filesystem at the given `store_path`, with
    /// specified lower and upper directories for the filesystem layers. It ensures the proper
    /// creation of directories and clears the contents of the upper layer if necessary.
    ///
    /// # Parameters
    /// - `inode`: The inode to associate with the overlay filesystem.
    /// - `store_path`: The path where the overlay filesystem should be mounted.
    ///
    /// # Returns
    /// A result indicating whether the mounting operation was successful.
    pub async fn overlay_mount<P: AsRef<Path>>(
        &self,
        inode: u64,
        store_path: P,
        need_mr: bool, // if need mr, then create mr layer.
    ) -> std::io::Result<()> {
        let lower = store_path.as_ref().join("lower");
        let upper = store_path.as_ref().join("upper");
        let mut lowerdir = vec![lower];
        if need_mr {
            let mr_path = store_path.as_ref().join("mr");
            let _ = std::fs::create_dir_all(&mr_path);
            lowerdir.push(mr_path);
        }
        let upperdir = upper;

        let config = config::Config {
            mountpoint: String::new(),
            do_import: true,
            ..Default::default()
        };
        // Create lower layers
        let mut lower_layers = Vec::new();
        for lower in &lowerdir {
            let lower_path = Path::new(lower);
            if lower_path.exists() {
                let layer = new_passthroughfs_layer(lower.to_str().unwrap()).await?;
                lower_layers.push(Arc::new(layer));
                // Rest of the code...
            } else {
                std::fs::create_dir_all(lower_path)?;
                let layer = new_passthroughfs_layer(lower.to_str().unwrap()).await?;
                lower_layers.push(Arc::new(layer));
            }
        }
        // Check if the upper directory exists
        let upper_path = Path::new(&upperdir);
        if !upper_path.exists() {
            // Create the upper directory if it doesn't exist
            std::fs::create_dir_all(&upperdir)?;
        } else {
            // Clear the contents of the upper directory`
            let entries = std::fs::read_dir(&upperdir)?;
            for entry in entries {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    std::fs::remove_dir_all(entry.path())?;
                } else {
                    std::fs::remove_file(entry.path())?;
                }
            }
        }
        // Create upper layer
        let upper_layer = Arc::new(new_passthroughfs_layer(upperdir.to_str().unwrap()).await?);
        let overlayfs = OverlayFs::new(Some(upper_layer), lower_layers, config, inode)?;
        self.overlayfs
            .lock()
            .await
            .insert(inode, Arc::new(overlayfs));
        self.after_mount_new().await;
        Ok(())
    }

    /// Unmounts the overlay filesystem associated with a given inode asynchronously.
    ///
    /// This function removes the overlay filesystem mapped to the specified inode from
    /// the `overlayfs` map and cleans up the associated resources.
    ///
    /// # Parameters
    /// - `inode`: The inode whose overlay filesystem is to be unmounted.
    ///
    /// # Returns
    /// A result indicating whether the unmounting operation was successful.
    pub async fn overlay_umount_byinode(&self, inode: u64) -> std::io::Result<()> {
        if !self.is_mount(inode).await {
            return Err(Error::new(
                std::io::ErrorKind::NotFound,
                "Overlay filesystem not mounted",
            ));
        }
        self.overlayfs.lock().await.remove(&inode);
        Ok(())
    }

    /// Unmounts the overlay filesystem associated with a given path asynchronously.
    ///
    /// This function retrieves the inode from the dictionary using the provided `path`
    /// and then calls `overlay_umount_byinode` to unmount the overlay filesystem.
    ///
    /// # Parameters
    /// - `path`: The path whose associated overlay filesystem is to be unmounted.
    ///
    /// # Returns
    /// A result indicating whether the unmounting operation was successful.
    pub async fn overlay_umount_bypath(&self, path: &str) -> std::io::Result<()> {
        let item = self.dic.store.get_by_path(path).await?;
        let inode = item.get_inode();
        self.overlay_umount_byinode(inode).await
    }

    /// Retrieves the inode associated with a given path asynchronously.
    ///
    /// This function queries the dictionary (`dic`) to obtain the inode associated
    /// with the specified `path`.
    ///
    /// # Parameters
    /// - `path`: The path whose inode is to be retrieved.
    ///
    /// # Returns
    /// A result containing the inode associated with the given path.
    pub async fn get_inode(&self, path: &str) -> std::io::Result<u64> {
        let item = self.dic.store.get_by_path(path).await?;
        Ok(item.get_inode())
    }

    /// Checks if an overlay filesystem is mounted for a given inode.
    ///
    /// This function checks if the `overlayfs` map contains the specified inode,
    /// indicating whether the overlay filesystem is currently mounted.
    ///
    /// # Parameters
    /// - `inode`: The inode to check for an associated mounted overlay filesystem.
    ///
    /// # Returns
    /// `true` if the overlay filesystem is mounted for the given inode, `false` otherwise.
    pub async fn is_mount(&self, inode: u64) -> bool {
        self.overlayfs.lock().await.get(&inode).is_some()
    }

    /// Allocates inode batches for every overlay filesystem asynchronously.
    ///
    /// This function clears the current inode allocation and then allocates new
    /// inode batches for all the overlay filesystems in the `overlayfs` map.
    ///
    /// # Returns
    /// None
    pub async fn after_mount_new(&self) {
        // clear inode alloc
        self.inodes_alloc.clear().await;
        // lock  overlayfs map
        let map_lock = &self.overlayfs.lock().await;

        for (inode, ovl_fs) in map_lock.iter() {
            // alloc new  inode batch.
            let inode_batch = self.inodes_alloc.alloc_inode(*inode).await;
            // extend inode alloc
            ovl_fs.extend_inode_alloc(inode_batch).await;
            // init overlay filesystem
            let _ = ovl_fs.init(Request::default()).await;
        }
    }
}
