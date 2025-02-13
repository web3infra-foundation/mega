

use fuse3::raw::{Filesystem, Request};
use inode_alloc::InodeAlloc;
use tokio::sync::Mutex;


use std::{collections::HashMap, io::Error, path::{Path, PathBuf}, sync::Arc};
use crate::{dicfuse::Dicfuse, manager::ScorpioManager, overlayfs::{config, OverlayFs}, passthrough::new_passthroughfs_layer};

mod inode_alloc;
mod async_io;

#[allow(unused)]
#[derive(Clone)]
pub struct MegaFuse{
    pub dic: Arc<Dicfuse>,
    overlayfs:Arc<Mutex<HashMap<u64,Arc<OverlayFs>>>>, // Inode -> overlayyfs 
    inodes_alloc: InodeAlloc,
   
}


#[allow(unused)]
impl MegaFuse{
    pub async fn new() -> Self{
        
        Self{
            
            dic: Arc::new(Dicfuse::new().await),
            overlayfs: Arc::new(Mutex::new(HashMap::new())),
            inodes_alloc: InodeAlloc::new(),
            
        }
    }
    pub async fn new_from_manager(manager: &ScorpioManager) -> MegaFuse {

        
        let megafuse = MegaFuse::new().await;
        //megafuse.dic.store.import().await;
        //For Buck path . associate with code-/src/dicfuse/store.rs:235 // import buck_out inode for buck2 , which inode number is 2.
        // let inode =megafuse.dic.store.get_by_path("buck_out").await.unwrap().get_inode();//buck out path .
        // println!("buck_out inode is :{}",inode);
        // let mut buck_path =PathBuf::from(&manager.store_path);
        // buck_path.push("buck_out");
        // std::fs::create_dir_all(&buck_path).unwrap();
        // megafuse.overlay_mount(inode, &buck_path).await;
        
        // mount user works.
        for dir in &manager.works {
            let _lower = PathBuf::from(&manager.store_path).join(&dir.hash);
            megafuse.overlay_mount(dir.node, &_lower).await;
        }


        megafuse
    }

    // TODO: add pass parameter: lower-dir and upper-dir.
    pub async  fn overlay_mount<P: AsRef<Path>>(&self, inode: u64, store_path: P) -> std::io::Result<()>{
        
        let lower = store_path.as_ref().join("lower");
        let upper = store_path.as_ref().join("upper");
        let lowerdir = vec![lower];
        let upperdir = upper;

        let config = config::Config {
            work: String::new(),
            mountpoint: String::new(),
            do_import: true,
            ..Default::default()
        };
        // Create lower layers
        let mut lower_layers = Vec::new();
        for lower in &lowerdir {
            let lower_path = Path::new(lower);
            if lower_path.exists() {
                let layer =
                    new_passthroughfs_layer(lower.to_str().unwrap()).await.unwrap();
                lower_layers.push(Arc::new(layer));
                // Rest of the code...
            } else {
                std::fs::create_dir_all(lower_path);
                let layer = new_passthroughfs_layer(lower.to_str().unwrap()).await.unwrap();
                lower_layers.push(Arc::new(layer));
            }
        }
        // Check if the upper directory exists
        let upper_path = Path::new(&upperdir);
        if !upper_path.exists() {
            // Create the upper directory if it doesn't exist
            std::fs::create_dir_all(&upperdir).unwrap();
        } else {
            // Clear the contents of the upper directory`
            let entries = std::fs::read_dir(&upperdir).unwrap();
            for entry in entries {
                let entry = entry.unwrap();
                std::fs::remove_file(entry.path()).unwrap();
            }
        }
        // Create upper layer
        let upper_layer = Arc::new(new_passthroughfs_layer(upperdir.to_str().unwrap()).await?);
        let overlayfs = OverlayFs::new(Some(upper_layer), lower_layers, config, inode)?;
        self.overlayfs.lock().await.insert(inode, Arc::new(overlayfs));
        self.after_mount_new().await;
        Ok(())
    }

    pub async fn overlay_umount_byinode(&self, inode:u64)  -> std::io::Result<()>{
        if !self.is_mount(inode).await{
            return Err( Error::new(std::io::ErrorKind::NotFound, "Overlay filesystem not mounted"))
        }
        self.overlayfs.lock().await.remove(&inode);
        Ok(())
    }
    
    pub async fn overlay_umount_bypath(&self, path:&str)-> std::io::Result<()>{
        let item = self.dic.store.get_by_path(path).await?;
        let inode = item.get_inode();
        self.overlay_umount_byinode(inode).await
    }
    pub async fn get_inode(&self,path:&str) ->std::io::Result<u64>{
        let item = self.dic.store.get_by_path(path).await?;
        Ok(item.get_inode())
    }
    pub async fn is_mount(&self,inode:u64) -> bool{
        self.overlayfs.lock().await.get(&inode).is_some()
    }
    // alloc inode batch number to every overlayfs .
    pub async fn after_mount_new(&self) {
        // clear inode alloc
        self.inodes_alloc.clear();
        // lock  overlayfs map
        let map_lock = &self.overlayfs.lock().await;
        
        for (inode, ovl_fs) in map_lock.iter() {
            // alloc new  inode batch.
            let inode_batch = self.inodes_alloc.alloc_inode(*inode).await;
            // extend inode alloc
            ovl_fs.extend_inode_alloc(inode_batch).await;
            // init overlay filesystem
            ovl_fs.init(Request::default()).await;
        }
    
    }
}
