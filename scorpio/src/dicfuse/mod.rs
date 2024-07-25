
pub mod model;
mod store;
mod fuse;

use std::{sync::Arc, time::Duration};
use std::io::Result;
use fuse_backend_rs::{abi::fuse_abi::FsOptions, api::filesystem::{Context, Entry, FileSystem}};
use tokio::task::JoinHandle;

use store::{DictionaryStore, IntoEntry};

struct Dicfuse{
    store: Arc<DictionaryStore>,
    //runtime: Arc<tokio::runtime::Runtime>,
}
#[allow(unused)]
impl Dicfuse{
    pub fn new() -> Self {
        Self {
            store: DictionaryStore::new().into(), // Assuming DictionaryStore has a new() method
            //runtime: tokio::runtime::Runtime::new().unwrap().into(), // Create a new runtime
        }
    }
    fn spawn<F, Fut, O>(&self, f: F) -> JoinHandle<O>
    where
        F: FnOnce(Arc<DictionaryStore>) -> Fut,
        Fut: std::future::Future<Output = O> + Send + 'static,
        O: Send + 'static,
    {
        let inner = self.store.clone();
        tokio::task::spawn(f(inner))
    }
}


#[allow(unused)]
impl FileSystem for Dicfuse{
    type Inode = u64;

    type Handle = u64;
    
    fn init(&self, capable:FsOptions) -> Result<FsOptions> {
        self.store.import();
        //let mut ops = FsOptions::DO_READDIRPLUS | FsOptions::READDIRPLUS_AUTO;
        Ok(fuse_backend_rs::abi::fuse_abi::FsOptions::empty())
    }
    
    fn destroy(&self) {}
    
    fn lookup(&self, ctx: &Context, parent: Self::Inode, name: &std::ffi::CStr) -> Result<Entry> {
        let store = self.store.clone();
        let mut ppath  = store.find_path(parent).ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENODATA))?;
        let pitem  = store.get_inode(parent)?;
        ppath.push(name.to_string_lossy().into_owned());
        let chil = store.get_by_path(&ppath.to_string())?;
        Ok(chil.into_entry())
    }
    

    fn forget(&self, ctx: &Context, inode: Self::Inode, count: u64) {}
    
    fn batch_forget(&self, ctx: &Context, requests: Vec<(Self::Inode, u64)>) {
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
        let store = self.store.clone();
        let i = store.find_path(inode).ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENODATA))?;
        let entry  = fuse::default_file_entry(inode);
        Ok((entry.attr,Duration::from_secs(u64::MAX)))
    }
    
    fn setattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        attr: libc::stat64,
        handle: Option<Self::Handle>,
        valid: fuse_backend_rs::abi::fuse_abi::SetattrValid,
    ) -> std::io::Result<(libc::stat64, std::time::Duration)> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
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
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn mkdir(
        &self,
        ctx: &Context,
        parent: Self::Inode,
        name: &std::ffi::CStr,
        mode: u32,
        umask: u32,
    ) -> std::io::Result<Entry> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }

    fn unlink(&self, ctx: &Context, parent: Self::Inode, name: &std::ffi::CStr) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn rmdir(&self, ctx: &Context, parent: Self::Inode, name: &std::ffi::CStr) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
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
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn link(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        newparent: Self::Inode,
        newname: &std::ffi::CStr,
    ) -> std::io::Result<Entry> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn open(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        flags: u32,
        fuse_flags: u32,
    ) -> std::io::Result<(Option<Self::Handle>, fuse_backend_rs::abi::fuse_abi::OpenOptions, Option<u32>)> {
        // Matches the behavior of libfuse.
        Ok((None, fuse_backend_rs::abi::fuse_abi::OpenOptions::empty(), None))
    }
    
    fn create(
        &self,
        ctx: &Context,
        parent: Self::Inode,
        name: &std::ffi::CStr,
        args: fuse_backend_rs::abi::fuse_abi::CreateIn,
    ) -> std::io::Result<(Entry, Option<Self::Handle>, fuse_backend_rs::abi::fuse_abi::OpenOptions, Option<u32>)> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn flush(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        lock_owner: u64,
    ) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn fsync(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        datasync: bool,
        handle: Self::Handle,
    ) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
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
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
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
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn statfs(&self, ctx: &Context, inode: Self::Inode) -> std::io::Result<libc::statvfs64> {
        // Safe because we are zero-initializing a struct with only POD fields.
        let mut st: libc::statvfs64 = unsafe { std::mem::zeroed() };
        // This matches the behavior of libfuse as it returns these values if the
        // filesystem doesn't implement this method.
        st.f_namemax = 255;
        st.f_bsize = 512;
        Ok(st)
    }
    
    fn setxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &std::ffi::CStr,
        value: &[u8],
        flags: u32,
    ) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn getxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        name: &std::ffi::CStr,
        size: u32,
    ) -> std::io::Result<fuse_backend_rs::api::filesystem::GetxattrReply> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn listxattr(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        size: u32,
    ) -> std::io::Result<fuse_backend_rs::api::filesystem::ListxattrReply> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn opendir(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        flags: u32,
    ) -> std::io::Result<(Option<Self::Handle>, fuse_backend_rs::abi::fuse_abi::OpenOptions)> {
        // Matches the behavior of libfuse.
        Ok((None, fuse_backend_rs::abi::fuse_abi::OpenOptions::empty()))
    }
    
    fn readdir(
        &self,
        ctx: &Context,
        inode: Self::Inode,
        handle: Self::Handle,
        size: u32,
        offset: u64,
        add_entry: &mut dyn FnMut(fuse_backend_rs::api::filesystem::DirEntry) -> std::io::Result<usize>,
    ) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
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
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    
    fn access(&self, ctx: &Context, inode: Self::Inode, mask: u32) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(libc::ENOSYS))
    }
    

}

#[cfg(test)]
mod tests {
    use std::{io, path::Path, sync::Arc,thread};

    use fuse_backend_rs::{ api::server::Server, transport::{FuseChannel, FuseSession}};
    use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};

    use super::Dicfuse;


    pub struct DicFuseServer {
        server: Arc<Server<Arc<Dicfuse>>>,
        ch: FuseChannel,
    }
    impl DicFuseServer {
        pub fn svc_loop(&mut self) -> Result<(),io::Error> {
            let _ebadf = std::io::Error::from_raw_os_error(libc::EBADF);
            println!("entering server loop");
            loop {
                if let Some((reader, writer)) = self
                    .ch
                    .get_request()
                    .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?
                {
                    if let Err(e) = self
                        .server
                        .handle_message(reader, writer.into(), None, None)
                    {
                        match e {
                            fuse_backend_rs::Error::EncodeMessage(_ebadf) => {
                                break;
                            }
                            _ => {
                                print!("Handling fuse message failed");
                                continue;
                            }
                        }
                    }
                } else {
                    print!("fuse server exits");
                    break;
                }
            }
            Ok(())
        }
    

    }

    #[test]
    fn test_svc_loop_success() {
        let dicfuse = Arc::new(Dicfuse::new());
       // dicfuse.init(FsOptions::empty()).unwrap();
        // Create fuse session
        let mut se = FuseSession::new(Path::new(&"/home/luxian/ccode/mega/dictest"), "dic", "", true).unwrap();
        se.mount().unwrap();
        let ch: FuseChannel = se.new_channel().unwrap();
        println!("start fs servers");
        let server = Arc::new(Server::new(dicfuse.clone()));

        let mut dicfuse_server = DicFuseServer { server, ch };

        // Spawn server thread
        let handle = thread::spawn(move || {
            let _ = dicfuse_server.svc_loop();
        });
        // Wait for termination signal
        let mut signals = Signals::new(TERM_SIGNALS).unwrap();
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
