use std::io::Result;
use fuse_backend_rs::{abi::fuse_abi::FsOptions, api::filesystem::{Context, Entry, FileSystem}};

// LoggingFileSystem . provide log info for a filesystem trait.
#[allow(unused)]
struct LoggingFileSystem<FS: FileSystem> {
    inner: FS,
}
#[allow(unused)]
impl<FS: FileSystem> LoggingFileSystem<FS> {
    // create a new  LoggingFileSystem wrapper
    pub fn new(inner: FS) -> Self {
        LoggingFileSystem { inner }
    }
}

// 为 LoggingFileSystem 实现 FileSystem trait
impl<FS: FileSystem<Handle = u64, Inode = u64 >> FileSystem for LoggingFileSystem<FS> {
    type Inode = FS::Inode;
    type Handle = FS::Handle ;
    fn readdir(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        size: u32,
        offset: u64,
        add_entry: &mut dyn FnMut(fuse_backend_rs::api::filesystem::DirEntry) -> std::io::Result<usize>,
    ) -> std::io::Result<()> {
        println!("[readdir]: inode:{},handle:{},size:{},offset:{}",inode,handle,size,offset);
        self.inner.readdir(ctx, inode, handle, size, offset, add_entry)
    }

    fn opendir(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        flags: u32,
    ) -> std::io::Result<(Option<Self::Handle>, fuse_backend_rs::abi::fuse_abi::OpenOptions)> {
        // Matches the behavior of libfuse.
        println!("[opendir]: ctx:{:?},inode:{},flags:{}", ctx,inode,flags);
        let re = self.inner.opendir(ctx, inode, flags);
        println!("[opendir-out]: {:?}", re);
        re
    }



    fn init(&self, capable:FsOptions) -> Result<FsOptions> {
        println!("Dicfuse init....");
        self.inner.init(capable)
    }
    
    fn destroy(&self) {}
    
    fn lookup(&self, ctx: &Context, parent: Self::Inode, name: &std::ffi::CStr) -> Result<Entry> {
        println!("[lookup]: ctx:{}, parnet inode:{},name :{:?}",ctx.pid,parent,name);
        self.inner.lookup(ctx, parent, name)
    }
    

    fn forget(&self, ctx: &Context, inode: Self::Inode, count: u64) {
        println!("[forget]: ctx:{}, inode:{},count :{}",ctx.pid,inode,count);
        self.inner.forget(ctx, inode, count)
    }
    
    fn batch_forget(&self, ctx: &Context, requests: Vec<(Self::Inode, u64)>) {
        println!("[batch-forget]: ctx:{}",ctx.pid);
        for (inode, count) in requests {
            self.forget(ctx, inode, count)
        }
    }
    
    fn getattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Option<Self::Handle>,
    ) -> std::io::Result<(libc::stat64, std::time::Duration)> {
        println!("[getattr]: ctx:{},  inode:{},handle :{:?}",ctx.pid,inode ,handle);
        let re = self.inner.getattr(ctx, inode, handle);
        println!("[getattr-out]:{:?}",re);
        re
    }
    
    fn setattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        attr: libc::stat64,
        handle: Option<Self::Handle>,
        valid: fuse_backend_rs::abi::fuse_abi::SetattrValid,
    ) -> std::io::Result<(libc::stat64, std::time::Duration)> {
        println!("[getattr]: ctx:{},  inode:{},handle :{:?}",ctx.pid,inode ,handle);
        self.inner.setattr(ctx, inode, attr, handle, valid)
    }
    
    
    fn mknod(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &std::ffi::CStr,
        mode: u32,
        rdev: u32,
        umask: u32,
    ) -> std::io::Result<Entry> {
        println!("[mknod]: ctx:{:?},  inode:{},name :{:?},umask:{}",ctx,inode,name,umask);
        self.inner.mknod(ctx, inode, name, mode, rdev, umask)
    }
    
    fn mkdir(
        &self,
        ctx: &Context,
        parent: Self::Inode,
        name: &std::ffi::CStr,
        mode: u32,
        umask: u32,
    ) -> std::io::Result<Entry> {
        println!("[mkdir]: parent:{},name:{:?},mode:{}",parent,name,mode);
        self.inner.mkdir(ctx, parent, name, mode, umask)
    }

    fn unlink(&self, ctx: &Context, parent: Self::Inode, name: &std::ffi::CStr) -> std::io::Result<()> {
        println!("[unlink]: parent:{},name:{:?}",parent,name);
        self.inner.unlink(ctx, parent, name)
    }

    
    #[inline]
    fn rmdir(&self, ctx: &Context, parent: Self::Inode, name: &std::ffi::CStr) -> std::io::Result<()> {
        println!("[rmdir]: parent:{},name:{:?}",parent,name);
        self.inner.rmdir(ctx, parent, name)
    }
    
    fn rename(
        &self,
        ctx: &Context,
        olddir: Self::Inode,
        oldname: &std::ffi::CStr,
        newdir: Self::Inode,
        newname: &std::ffi::CStr,
        flags: u32,
    ) -> std::io::Result<()> {
        println!("[rename]: not implement.");
        self.inner.rename(ctx, olddir, oldname, newdir, newname, flags)
    }
    
    fn link(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        newparent: Self::Inode,
        newname: &std::ffi::CStr,
    ) -> std::io::Result<Entry> {
        println!("[link]: not implement.");
        self.inner.link(ctx, inode, newparent, newname)
    }
    
    fn open(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        flags: u32,
        fuse_flags: u32,
    ) -> std::io::Result<(Option<Self::Handle>, fuse_backend_rs::abi::fuse_abi::OpenOptions, Option<u32>)> {
        println!("[open]: inode:{}",inode);
        // Matches the behavior of libfuse.
       self.inner.open(ctx, inode, flags, fuse_flags)
    }
    
    fn create(
        &self,
        ctx: &Context,
        parent: Self::Inode,
        name: &std::ffi::CStr,
        args: fuse_backend_rs::abi::fuse_abi::CreateIn,
    ) -> std::io::Result<(Entry, Option<Self::Handle>, fuse_backend_rs::abi::fuse_abi::OpenOptions, Option<u32>)> {
        println!("[create]: parnet:{},name:{:?}",parent,name);
        self.inner.create(ctx, parent, name, args)
    }
    
    fn flush(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        lock_owner: u64,
    ) -> std::io::Result<()> {
        println!("[flush]: not implement.");
        self.inner.flush(ctx, inode, handle, lock_owner)
    }
    
    fn fsync(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        datasync: bool,
        handle: Self::Handle,
    ) -> std::io::Result<()> {
        println!("[fsync]: not implement.");
        self.inner.fsync(ctx, inode, datasync, handle)
    }
    
    fn fallocate(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        mode: u32,
        offset: u64,
        length: u64,
    ) -> std::io::Result<()> {
        println!("[fallocate]: not implement.");
        self.inner.fallocate(ctx, inode, handle, mode, offset, length)
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
    ) -> std::io::Result<()> {
        println!("[release]: not implement.");
        self.inner.release(ctx, inode, flags, handle, flush, flock_release, lock_owner)
    }
    
    fn statfs(&self, ctx: &Context, inode: Self::Inode) -> std::io::Result<libc::statvfs64> {
        println!("[statfs]");
        self.inner.statfs(ctx, inode)
    }
    
    fn setxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &std::ffi::CStr,
        value: &[u8],
        flags: u32,
    ) -> std::io::Result<()> {
        println!("[setxattr]: not implement.");
        self.inner.setxattr(ctx, inode, name, value, flags)
    }
    
    fn getxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &std::ffi::CStr,
        size: u32,
    ) -> std::io::Result<fuse_backend_rs::api::filesystem::GetxattrReply> {
        println!("[getxattr]: inode:{},name:{:?},size{}", inode,name,size);
       self.inner.getxattr(ctx, inode, name, size)
    }
    
    fn listxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        size: u32,
    ) -> std::io::Result<fuse_backend_rs::api::filesystem::ListxattrReply> {
        println!("[listxattr]: not implement.");
        self.inner.listxattr(ctx, inode, size)
    }
    

    
    fn readdirplus(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        size: u32,
        offset: u64,
        add_entry: &mut dyn FnMut(fuse_backend_rs::api::filesystem::DirEntry, Entry) -> std::io::Result<usize>,
    ) -> std::io::Result<()> {
        println!("[readdirplus]: inode:{}, handle:{:?},size:{},offset:{}",inode,handle,size,offset);
        self.inner.readdirplus(ctx, inode, handle, size, offset, add_entry)
    }
    
    fn access(&self, ctx: &Context, inode: Self::Inode, mask: u32) -> std::io::Result<()> {
        println!("[access]: not implement.");
        self.inner.access(ctx, inode, mask)
    }
}


#[cfg(test)]
mod tests{
    use std::{path::Path, sync::Arc, thread};

    use fuse_backend_rs::{api::server::Server, transport::{FuseChannel, FuseSession}};
    use signal_hook::iterator::Signals;

    use crate::{passthrough, server::FuseServer};
    use super::LoggingFileSystem;
    
    #[test]
    fn test_tracerse_drectory(){
        let config = fuse_backend_rs::passthrough::Config { 
            root_dir: String::from("/home/luxian/megatest/lower"), 
            // enable xattr`
            xattr: true, 
            do_import: true, 
            ..Default::default() };


        let fs: LoggingFileSystem<fuse_backend_rs::passthrough::PassthroughFs> = LoggingFileSystem::new(passthrough::passthrough::PassthroughFs::<()>::new(config).unwrap());

        // Create fuse session
        let mut se = FuseSession::new(Path::new(&"/home/luxian/megatest/dictest"), "td", "", false).unwrap();
        se.mount().unwrap();
        let ch: FuseChannel = se.new_channel().unwrap();


        let mut server = FuseServer{
            server:  Arc::new(Server::new(fs)),
            ch
        };
        // Spawn server thread
        let handle = thread::spawn(move || {
            let _ = server.svc_loop();
        });


        // Wait for termination signal
        let mut signals = Signals::new(signal_hook::consts::TERM_SIGNALS).unwrap();
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