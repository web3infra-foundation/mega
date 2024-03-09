use super::inode::*;
use crate::common::{FMODE_EXEC, MAX_NAME_LENGTH};
use crate::local_tmp::TemporaryFileManager;
use crate::req_remote::RemoteServer;
use fuser::consts::FOPEN_DIRECT_IO;
use fuser::TimeOrNow::Now;
use fuser::{
    FileType, Filesystem, KernelConfig, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyWrite, Request, TimeOrNow, FUSE_ROOT_ID,
};
use simple_log::{debug, error, info, warn};
use std::collections::{HashMap, LinkedList};
use std::ffi::OsStr;
use std::os::raw::c_int;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::runtime::Runtime;
use tokio::signal::unix::{signal, SignalKind};

pub struct RLFileSystem {
    fs_name: String,
    remote_root: String,
    tmp_manager: TemporaryFileManager,
    inodes: HashMap<u64, Inode>,
    remote: RemoteServer,
    lock: Mutex<()>,
    direct_io: bool,
    rt: Arc<Runtime>,
}

impl RLFileSystem {
    pub fn new(
        remote_url: String,
        fs_name: String,
        direct_io: bool,
        remote_root: String,
        data_dir: String,
    ) -> Self {
        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        );
        Self {
            fs_name,
            rt: rt.clone(),
            tmp_manager: TemporaryFileManager::new(data_dir),
            inodes: HashMap::new(),
            remote: RemoteServer::new(remote_url, rt),
            lock: Mutex::new(()),
            direct_io,
            remote_root,
        }
    }

    pub fn lookup_name(&self, parent: u64, name: &str) -> Option<u64> {
        let parent_inode = self.inodes.get(&parent).unwrap();
        for ino in parent_inode.children_ino.iter() {
            let inode = self.inodes.get(ino).unwrap();
            if inode.attr.name.eq(name) {
                return Some(*ino);
            }
        }
        None
    }

    fn commit_signal(&mut self) {
        self.rt.block_on(async {
            debug!("commit all change.");
            let mut sig = signal(SignalKind::user_defined1()).unwrap();
            loop {
                sig.recv().await;
                let content = self.tmp_manager.generate_seq();
                self.remote.commit_change(content).await;
            }
        });
    }
}

impl Filesystem for RLFileSystem {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), c_int> {
        info!("init() -> Initialize filesystem.");
        let guard = self.lock.lock().unwrap();
        self.inodes.insert(FUSE_ROOT_ID, root_node(&self.fs_name));
        let mut queue = LinkedList::from([FUSE_ROOT_ID]);
        while let Some(ino) = queue.pop_front() {
            let inode = self.inodes.get_mut(&ino).unwrap();
            let path = inode.attr.path.clone();

            if let Some(metadata) = self.remote.list(self.remote_root.clone() + "/" + &path) {
                let new_inodes: Vec<Inode> = metadata
                    .into_iter()
                    .map(|content| {
                        let attr = InodeAttributes::from(content);
                        let kind = attr.kind;
                        let new_inode = Inode::new(ino, attr);
                        if kind == InodeKind::Directory {
                            queue.push_back(new_inode.ino);
                        }
                        inode.insert_child(new_inode.ino);
                        new_inode
                    })
                    .collect();
                new_inodes.into_iter().for_each(|new_inode| {
                    self.inodes.insert(new_inode.ino, new_inode);
                });
            } else {
                error!("Network error, file system initialization failed!");
                return Err(libc::NFT_PAYLOAD_NETWORK_HEADER);
            }
        }
        drop(guard);
        self.commit_signal();
        info!("File system init success.");
        Ok(())
    }

    fn destroy(&mut self) {
        info!("destroy() -> Clean up filesystem.");
        self.tmp_manager.clean_temp();
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if name.len() > MAX_NAME_LENGTH as usize {
            reply.error(libc::ENAMETOOLONG);
            return;
        }
        let name = name.to_str().unwrap().to_owned();
        debug!(
            "lookup() -> Look up a directory entry and get its attributes. {}",
            name.clone()
        );
        match self.lookup_name(parent, &name) {
            Some(ino) => {
                let inode = self.inodes.get(&ino).unwrap();
                reply.entry(&Duration::new(0, 0), &inode.file_attr(), 0)
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        match self.inodes.get(&ino) {
            Some(inode) => {
                debug!(
                    "getattr() -> Get file attributes. {}",
                    inode.attr.name.clone()
                );
                reply.attr(&Duration::new(0, 0), &inode.file_attr())
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let inode = self.inodes.get_mut(&ino).unwrap();
        if let Some(mode) = mode {
            inode.attr.permissions = mode as u16;
        }
        if let Some(size) = size {
            inode.attr.size = size;
        }
        if let Some(mtime) = mtime {
            inode.attr.mtime = match mtime {
                TimeOrNow::SpecificTime(time) => time,
                Now => SystemTime::now(),
            };
        }
        debug!(
            "setattr() -> Set file attributes. {}",
            inode.attr.name.clone()
        );
        self.tmp_manager.ops_alter(inode.attr.clone());
        reply.attr(&Duration::new(0, 0), &inode.file_attr());
    }

    fn mknod(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        let name = name.to_str().unwrap().to_owned();
        debug!("mknod() -> Create file node. {}", name.clone());
        if self.lookup_name(parent, &name).is_some() {
            reply.error(libc::EEXIST);
            return;
        }

        let parent_inode = self.inodes.get_mut(&parent).unwrap();
        let path = parent_inode.attr.path.clone() + "/" + &name;

        let file_type = mode & libc::S_IFMT;
        let kind = if file_type == libc::S_IFREG {
            InodeKind::File
        } else if file_type == libc::S_IFDIR {
            InodeKind::Directory
        } else {
            warn!("mknod() -> Implementation is incomplete. Only supports regular files, symlinks, and directories. Got {:o}", mode);
            reply.error(libc::ENOSYS);
            return;
        };
        let attr = InodeAttributes::new(name, kind, path);
        let new_inode = Inode::new(parent, attr.clone());
        match kind {
            InodeKind::Directory => self.tmp_manager.new_dir(new_inode.ino, attr.clone()),
            InodeKind::File => self.tmp_manager.new_file(new_inode.ino, attr.clone()),
        }
        parent_inode.attr.mtime = SystemTime::now();
        parent_inode.insert_child(new_inode.ino);
        self.tmp_manager.ops_alter(parent_inode.attr.clone());
        self.inodes.insert(new_inode.ino, new_inode.clone());
        reply.entry(&Duration::new(0, 0), &new_inode.file_attr(), 0);
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let name = name.to_str().unwrap().to_owned();
        debug!("mkdir() -> Create a directory. {}", name);
        if self.lookup_name(parent, &name).is_some() {
            reply.error(libc::EEXIST);
            return;
        }

        let parent_inode = self.inodes.get_mut(&parent).unwrap();
        parent_inode.attr.mtime = SystemTime::now();
        self.tmp_manager.ops_alter(parent_inode.attr.clone());
        let path = parent_inode.attr.path.clone() + "/" + &name;
        let attr = InodeAttributes::new(name, InodeKind::Directory, path);
        let new_inode = Inode::new(parent, attr.clone());
        self.tmp_manager.new_dir(new_inode.ino, attr);
        parent_inode.insert_child(new_inode.ino);
        reply.entry(&Duration::new(0, 0), &new_inode.file_attr(), 0);
        self.inodes.insert(new_inode.ino, new_inode);
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name = name.to_str().unwrap().to_owned();
        debug!("unlink() -> Remove a file. {}", name);
        let ino = match self.lookup_name(parent, &name) {
            Some(ino) => ino,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        let parent_inode = self.inodes.get_mut(&parent).unwrap();
        parent_inode.attr.mtime = SystemTime::now();
        parent_inode.remove_child(ino);
        self.tmp_manager.ops_alter(parent_inode.attr.clone());
        let inode = self.inodes.remove(&ino).unwrap();
        self.tmp_manager.rm_file(ino, inode.attr.id);
        reply.ok();
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let name = name.to_str().unwrap().to_string();
        debug!("rmdir() -> Remove a directory. {}", name);
        let ino = match self.lookup_name(parent, &name) {
            Some(ino) => ino,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let inode = self.inodes.get(&ino).unwrap();
        if !inode.children_ino.is_empty() {
            reply.error(libc::ENOTEMPTY);
            return;
        }
        let parent_inode = self.inodes.get_mut(&parent).unwrap();
        let mut index = 0;
        for (i, child_ino) in parent_inode.children_ino.iter().enumerate() {
            if *child_ino == ino {
                index = i;
            }
        }
        parent_inode.children_ino.remove(index);
        parent_inode.attr.mtime = SystemTime::now();
        self.tmp_manager.ops_alter(parent_inode.attr.clone());
        let inode = self.inodes.remove(&ino).unwrap();
        self.tmp_manager.rm_dir(ino, inode.attr.path);
        reply.ok();
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _newparent: u64,
        newname: &OsStr,
        flags: u32,
        reply: ReplyEmpty,
    ) {
        let new_name = newname.to_str().unwrap().to_string();
        if self.lookup_name(parent, &new_name).is_some() {
            reply.error(libc::EEXIST);
            return;
        }
        let name = name.to_str().unwrap().to_string();
        debug!("rename() -> Rename a file. {}->{}", name, new_name);
        let inode = match self.lookup_name(parent, &name) {
            Some(ino) => self.inodes.get_mut(&ino).unwrap(),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        if flags & libc::RENAME_EXCHANGE != 0 {
            inode.attr.name = new_name;
            inode.attr.mtime = SystemTime::now();
            self.tmp_manager.ops_alter(inode.attr.clone());
            reply.ok();
        }
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        let _ = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                if flags & FMODE_EXEC != 0 {
                    (libc::X_OK, true, false)
                } else {
                    (libc::R_OK, true, false)
                }
            }
            libc::O_WRONLY => (libc::W_OK, false, true),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true),
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let inode = self.inodes.get_mut(&ino).unwrap();
        debug!("open() -> Open a file. {}", inode.attr.name.clone());
        let open_flags = if self.direct_io { FOPEN_DIRECT_IO } else { 0 };
        if !self.tmp_manager.exist(ino) {
            let content = self.remote.download(inode.attr.id.clone()).unwrap();
            if !content.is_empty() {
                let mut bytes = Vec::new();
                content
                    .into_iter()
                    .for_each(|item| bytes.extend(item.to_vec()));
                self.tmp_manager.tmp_file(ino, &bytes);
            }
        }
        reply.opened(ino, open_flags);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        assert!(offset >= 0);
        if (fh & ino) != 0 {
            reply.error(libc::EACCES);
            return;
        }
        let file_size = match self.inodes.get(&ino) {
            Some(inode) => {
                debug!("read() -> Read data. {}", inode.attr.name.clone());
                inode.attr.size
            }
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let read_size = std::cmp::min(size, file_size.saturating_sub(offset as u64) as u32);
        let mut buf = Vec::new();
        self.tmp_manager.read(ino, &mut buf, read_size as u64);
        reply.data(&buf);
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        assert!(offset >= 0);

        let inode = match self.inodes.get_mut(&ino) {
            Some(inode) => {
                debug!("write() -> Write data. {}", inode.attr.name.clone());
                inode
            }
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };
        self.tmp_manager
            .append_content(ino, data, inode.attr.id.clone());
        inode.attr.mtime = SystemTime::now();
        if data.len() + offset as usize > inode.attr.size as usize {
            inode.attr.size = (data.len() + offset as usize) as u64;
        }
        self.tmp_manager.ops_alter(inode.attr.clone());
        reply.written(data.len() as u32);
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                (libc::R_OK, true, false)
            }
            libc::O_WRONLY => (libc::W_OK, false, true),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true),
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };
        let open_flags = if self.direct_io { FOPEN_DIRECT_IO } else { 0 };
        reply.opened(ino, open_flags);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let inode = self.inodes.get(&ino).unwrap();
        let mut entires = vec![
            (ino, FileType::Directory, ".".to_owned()),
            (ino, FileType::Directory, "..".to_owned()),
        ];
        let children: Vec<(u64, FileType, String)> = inode
            .children_ino
            .iter()
            .map(|ino| self.inodes.get(ino).unwrap())
            .map(|inode| (inode.ino, inode.attr.kind.into(), inode.attr.name.clone()))
            .collect();
        entires.extend(children);

        for (index, (ino, kind, name)) in entires.into_iter().enumerate().skip(offset as usize) {
            if reply.add(ino, (index + 1) as i64, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        let name = name.to_str().unwrap().to_string();
        debug!("create() -> Create and open a file. {}",name);
        if self.lookup_name(parent, &name).is_some() {
            reply.error(libc::EEXIST);
            return;
        }

        match flags & libc::O_ACCMODE {
            libc::O_RDONLY => (true, false),
            libc::O_WRONLY => (false, true),
            libc::O_RDWR => (true, true),
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let parent_inode = self.inodes.get_mut(&parent).unwrap();
        parent_inode.attr.mtime = SystemTime::now();
        self.tmp_manager.ops_alter(parent_inode.attr.clone());
        let path = parent_inode.attr.path.clone() + "/" + &name;
        let attr = InodeAttributes::new(name, InodeKind::File, path);
        let new_inode = Inode::new(parent, attr.clone());
        let new_ino = new_inode.ino;
        self.tmp_manager.new_file(new_ino, attr.clone());
        parent_inode.insert_child(new_ino);
        self.inodes.insert(new_inode.ino, new_inode.clone());
        reply.created(&Duration::new(0, 0), &new_inode.file_attr(), 0, new_ino, 0);
    }

    fn fallocate(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        length: i64,
        mode: i32,
        reply: ReplyEmpty,
    ) {
        match self.inodes.get_mut(&ino) {
            Some(inode) => {
                debug!("fallocate() -> Preallocate or deallocate space to a file. {}",inode.attr.name.clone());
                self.tmp_manager
                    .fallocate(ino, mode, offset, length, inode.attr.id.clone());
                if mode & libc::FALLOC_FL_KEEP_SIZE == 0 {
                    inode.attr.mtime = SystemTime::now();
                    if (offset + length) as u64 > inode.attr.size {
                        inode.attr.size = (offset + length) as u64;
                    }
                }
                self.tmp_manager.ops_alter(inode.attr.clone());
            }
            None => reply.ok(),
        }
    }
}
