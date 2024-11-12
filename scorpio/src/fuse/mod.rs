
use fuse3::raw::{Filesystem, Request};
use inode_alloc::InodeAlloc;
use tokio::sync::Mutex;


use std::{collections::HashMap,  path::{Path, PathBuf}, sync::Arc};
use crate::{dicfuse::Dicfuse, manager::ScorpioManager, overlayfs::{config, OverlayFs}, passthrough::new_passthroughfs_layer};

mod inode_alloc;
mod async_io;

#[allow(unused)]
#[derive(Clone)]
pub struct MegaFuse{
    dic: Arc<Dicfuse>,
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
        for dir in &manager.works {
            let _lower = PathBuf::from(&manager.store_path).join(&dir.hash);
            megafuse.overlay_mount(dir.node, &_lower).await;
        }
        megafuse
    }

    // TODO: add pass parameter: lower-dir and upper-dir.
    pub async  fn overlay_mount<P: AsRef<Path>>(&self, inode: u64, store_path: P){
        
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
                panic!("Lower directory does not exist: {}", lower.to_str().unwrap());
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
        let upper_layer = Arc::new(new_passthroughfs_layer(upperdir.to_str().unwrap()).await.unwrap());
        let overlayfs = OverlayFs::new(Some(upper_layer), lower_layers, config, inode).unwrap();
        self.overlayfs.lock().await.insert(inode, Arc::new(overlayfs));
    }

    pub async fn overlay_un_mount<P: AsRef<Path>>(&self,  store_path: P)  -> std::io::Result<()>{
        
        Ok(())
    }
    
    pub async fn get_inode(&self,path:&str) -> u64{
        let item = self.dic.store.get_by_path(path).await;
        item.unwrap().get_inode()
    }
    pub async fn async_init(&self){
        self.dic.store.async_import().await;
        let map_lock = &self.overlayfs.lock().await;
        for (inode,ovl_fs) in map_lock.iter(){
            let inode_batch = self.inodes_alloc.alloc_inode(*inode).await;
            ovl_fs.extend_inode_alloc(inode_batch).await;
            ovl_fs.init(Request::default()).await;
        }
    
    }
}
