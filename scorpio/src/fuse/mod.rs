use fuse_backend_rs::{abi::fuse_abi::{CreateIn, FsOptions, OpenOptions, SetattrValid}, api::filesystem::{Context, DirEntry, Entry, FileSystem, GetxattrReply, Layer, ListxattrReply, ZeroCopyReader, ZeroCopyWriter}};
use inode_alloc::InodeAlloc;
use libc::{stat64, statvfs64};
use std::{collections::HashMap, ffi::CStr, io::Result, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::Duration};
use crate::{dicfuse::Dicfuse, manager::ScorpioManager, overlayfs::{config, OverlayFs}, passthrough::new_passthroughfs_layer};

mod inode_alloc;

pub use inode_alloc::READONLY_INODE;
#[allow(unused)]
pub struct MegaFuse{
    dic: Arc<Dicfuse>,
    overlayfs:Mutex<HashMap<u64,Arc<OverlayFs>>>, // Inode -> overlayyfs 
    inodes_alloc: InodeAlloc,
}
/// select the right fs by inodes .
/// 1. in inodes < READONLY_INODE , it from the readonly fuse.
/// 2. if inodes is a overlay fs root ,find it from the hashmap
/// 3. if inode from one  overlay, find it by batch number.
macro_rules! select_filesystem {
   
    ($self:ident, $inode:expr) => {
        if let Some(ovl_inode) = $self.inodes_alloc.get_ovl_inode($inode/READONLY_INODE) {
            if let Some(ovl_inode_root) = $self.overlayfs.lock().unwrap().get(&ovl_inode){
                println!(" overlay child inode root");
                ovl_inode_root.clone()
            }else{
                panic!("can't find fs by inode");
            }
        }else if let Some(ovl_inode_root) = $self.overlayfs.lock().unwrap().get(&$inode){
            println!(" overlay inode root");
            ovl_inode_root.clone()
        }else if ($inode < READONLY_INODE){
            println!(" readonly inode root");
            $self.dic.clone()
        }else{
            panic!("can't find fs by inode");
        }
    }
}

impl Default for MegaFuse {
    fn default() -> Self {
        Self::new()
    }
}
#[allow(unused)]
impl MegaFuse{
    pub fn new() -> Self{
        Self{
            dic: Arc::new(Dicfuse::new()),
            overlayfs: Mutex::new(HashMap::new()) ,
            inodes_alloc: InodeAlloc::new(),
        }
    }
    pub fn new_from_manager(manager: &ScorpioManager) -> Arc<MegaFuse> {
        let megafuse = Arc::new(MegaFuse::new());
        for dir in &manager.works {
            let _lower = PathBuf::from(&manager.store_path).join(&dir.hash);
            megafuse.overlay_mount(dir.node, &_lower);
        }
        megafuse
    }

    // TODO: add pass parameter: lower-dir and upper-dir.
    fn overlay_mount<P: AsRef<Path>>(&self, inode: u64, store_path: P) {
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
                let layer: Box<dyn Layer<Inode = u64, Handle = u64> + Send + Sync> =
                    new_passthroughfs_layer(lower.to_str().unwrap()).unwrap();
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
            // Clear the contents of the upper directory
            let entries = std::fs::read_dir(&upperdir).unwrap();
            for entry in entries {
                let entry = entry.unwrap();
                std::fs::remove_file(entry.path()).unwrap();
            }
        }
        // Create upper layer
        let upper_layer = Arc::new(new_passthroughfs_layer(upperdir.to_str().unwrap()).unwrap());
        let overlayfs = OverlayFs::new(Some(upper_layer), lower_layers, config, inode).unwrap();

        self.overlayfs.lock().unwrap().insert(inode, Arc::new(overlayfs));
    }

}
impl FileSystem for MegaFuse{
    type Inode = u64;
    type Handle = u64;
    fn init(&self, capable: FsOptions) -> Result<FsOptions> {
        self.dic.init(capable).unwrap();
        let map_lock = &self.overlayfs.lock().unwrap();
        for (inode,ovl_fs) in map_lock.iter(){
            let inode_batch = self.inodes_alloc.alloc_inode(*inode);
            ovl_fs.extend_inode_alloc(inode_batch);
            ovl_fs.init(capable).unwrap();
        }
        Ok(fuse_backend_rs::abi::fuse_abi::FsOptions::empty())
    }

    // fn destroy(&self) {}

    fn statfs(&self, ctx: &Context, inode: Self::Inode) -> Result<statvfs64> {

        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.statfs(ctx, inode)
    }

    fn lookup(&self, ctx: &Context, parent: Self::Inode, name: &CStr) -> Result<Entry> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,parent );
        a.lookup(ctx, parent, name)
    }

    fn opendir(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        flags: u32,
    ) -> Result<(Option<Self::Handle>, OpenOptions)> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.opendir(ctx, inode, flags)
    }

    fn releasedir(&self, ctx: &Context, inode: Self::Inode, flags: u32, handle: Self::Handle) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.releasedir(ctx, inode, flags, handle)
    }

    // for mkdir or create file
    // 1. lookup name, if exists and not whiteout, return EEXIST
    // 2. not exists and no whiteout, copy up parent node, ususally  a mkdir on upper layer would do the work
    // 3. find whiteout, if whiteout in upper layer, should set opaque. if in lower layer, just mkdir?
    fn mkdir(
        &self,
        ctx: &Context,
        parent: Self::Inode,
        name: &CStr,
        mode: u32,
        umask: u32,
    ) -> Result<Entry> {
        println!("top mkdir : parent:{}, name :{:?}, mode:{},umask:{}",parent,name,mode,umask);
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,parent );
        a.mkdir(ctx, parent, name, mode, umask)
    }

    fn rmdir(&self, ctx: &Context, parent: Self::Inode, name: &CStr) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,parent );
        a.rmdir(ctx, parent, name)
    }

    fn readdir(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        size: u32,
        offset: u64,
        add_entry: &mut dyn FnMut(DirEntry) -> Result<usize>,
    ) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.readdir(ctx, inode, handle, size, offset, add_entry)
    }

    fn readdirplus(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        size: u32,
        offset: u64,
        add_entry: &mut dyn FnMut(DirEntry, Entry) -> Result<usize>,
    ) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.readdirplus(ctx, inode, handle, size, offset, add_entry)
    }

    fn open(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        flags: u32,
        fuse_flags: u32,
    ) -> Result<(Option<Self::Handle>, OpenOptions, Option<u32>)> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.open(ctx, inode, flags, fuse_flags)
    }

    fn release(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        flags: u32,
        handle: Self::Handle,
        flush: bool,
        flock_release: bool,
        lock_owner: Option<u64>,
    ) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.release(ctx, inode, flags, handle, flush, flock_release, lock_owner)
    }

    fn create(
        &self,
        ctx: &Context,
        parent: Self::Inode,
        name: &CStr,
        args: CreateIn,
    ) -> Result<(Entry, Option<Self::Handle>, OpenOptions, Option<u32>)> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,parent );
        a.create(ctx, parent, name, args)
    }

    fn unlink(&self, ctx: &Context, parent: Self::Inode, name: &CStr) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,parent );
        a.unlink(ctx, parent, name)
    }

    fn read(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        w: &mut dyn ZeroCopyWriter,
        size: u32,
        offset: u64,
        lock_owner: Option<u64>,
        flags: u32,
    ) -> Result<usize> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.read(ctx, inode, handle, w, size, offset, lock_owner, flags)
    }

    fn write(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        r: &mut dyn ZeroCopyReader,
        size: u32,
        offset: u64,
        lock_owner: Option<u64>,
        delayed_write: bool,
        flags: u32,
        fuse_flags: u32,
    ) -> Result<usize> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.write(ctx, inode, handle, r, size, offset, lock_owner, delayed_write, flags, fuse_flags)
    }

    fn getattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Option<Self::Handle>,
    ) -> Result<(stat64, Duration)> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.getattr(ctx, inode, handle)
    }

    fn setattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        attr: stat64,
        handle: Option<Self::Handle>,
        valid: SetattrValid,
    ) -> Result<(stat64, Duration)> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.setattr(ctx, inode, attr, handle, valid)
    }



    fn mknod(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &CStr,
        mode: u32,
        rdev: u32,
        umask: u32,
    ) -> Result<Entry> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.mknod(ctx, inode, name, mode, rdev, umask)
    }

    fn link(&self, ctx: &Context, inode: Self::Inode, newparent: Self::Inode, name: &CStr) -> Result<Entry> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.link(ctx, inode, newparent, name)
    }

    fn symlink(&self, ctx: &Context, linkname: &CStr, parent: Self::Inode, name: &CStr) -> Result<Entry> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,parent );
        a.symlink(ctx, linkname, parent, name)
    }

    fn readlink(&self, ctx: &Context, inode: Self::Inode) -> Result<Vec<u8>> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.readlink(ctx, inode)
    }

    fn flush(&self, ctx: &Context, inode: Self::Inode, handle: Self::Handle, lock_owner: u64) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.flush(ctx, inode, handle, lock_owner)
    }

    fn fsync(&self, ctx: &Context, inode: Self::Inode, datasync: bool, handle: Self::Handle) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.fsync(ctx, inode, datasync, handle)
    }

    fn fsyncdir(&self, ctx: &Context, inode: Self::Inode, datasync: bool, handle: Self::Handle) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.fsyncdir(ctx, inode, datasync, handle)
    }

    fn access(&self, ctx: &Context, inode: Self::Inode, mask: u32) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.access(ctx, inode, mask)
    }

    fn setxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &CStr,
        value: &[u8],
        flags: u32,
    ) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.setxattr(ctx, inode, name, value, flags)
    }

    fn getxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &CStr,
        size: u32,
    ) -> Result<GetxattrReply> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.getxattr(ctx, inode, name, size)
    }

    fn listxattr(&self, ctx: &Context, inode: Self::Inode, size: u32) -> Result<ListxattrReply> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.listxattr(ctx, inode, size)
    }

    fn removexattr(&self, ctx: &Context, inode: Self::Inode, name: &CStr) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.removexattr(ctx, inode, name)
    }

    fn fallocate(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        mode: u32,
        offset: u64,
        length: u64,
    ) -> Result<()> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.fallocate(ctx, inode, handle, mode, offset, length)
    }

    fn lseek(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        offset: u64,
        whence: u32,
    ) -> Result<u64> {
        let a:Arc<dyn FileSystem<Inode = u64,Handle = u64>> = select_filesystem!(self,inode );
        a.lseek(ctx, inode, handle, offset, whence)
    }
}



#[cfg(test)]
mod tests{
    use std::{path::Path, thread};

    use crate::server::FuseServer;

    use super::*;
    use fuse_backend_rs::{api::server::Server, transport::{FuseChannel, FuseSession}};
    use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};
    
    #[test]
    pub fn test_dic_ovlfs(){
        let megafuse = Arc::new(MegaFuse::new());
       // dicfuse.init(FsOptions::empty()).unwrap();
        // Create fuse session
        let mut se = FuseSession::new(Path::new(&"/home/luxian/megatest/dictest"), "dic", "", false).unwrap();
        se.mount().unwrap();
        let ch: FuseChannel = se.new_channel().unwrap();
        println!("start fs servers");
        let server = Arc::new(Server::new(megafuse.clone()));

        let mut fuse_server = FuseServer { server, ch };

        // Spawn server thread
        let handle = thread::spawn(move || {
            let _ = fuse_server.svc_loop();
        });
        // Wait for termination signal
        let mut signals = Signals::new(TERM_SIGNALS).unwrap();
        println!("Signals start");
        if let Some(_sig) = signals.forever().next() {
            //pass
        }
        // Unmount and wake up
        se.umount().unwrap();
        se.wake().unwrap();
        // Join server thread
        let _ = handle.join();
    }
}