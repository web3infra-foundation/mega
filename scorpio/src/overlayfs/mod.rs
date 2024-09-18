// Copyright (C) 2023 Ant Group. All rights reserved.
//  2024 From [fuse_backend_rs](https://github.com/cloud-hypervisor/fuse-backend-rs) 
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
pub mod config;
mod inode_store;
pub mod sync_io;
mod utils;
mod diff;
mod tempfile;
use core::panic;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io::{Error, ErrorKind, Result, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};

use config::Config;
use fuse_backend_rs::abi::fuse_abi::{stat64, statvfs64, CreateIn};
use fuse_backend_rs::api::filesystem::{
    Context, DirEntry, Entry, Layer, OpenOptions
};
#[cfg(not(feature = "async-io"))]
use fuse_backend_rs::api::BackendFileSystem;
use fuse_backend_rs::api::{SLASH_ASCII, VFS_MAX_INO};
use inode_store::InodeStore;


pub type Inode = u64;
pub type Handle = u64;
pub const CURRENT_DIR: &str = ".";
pub const PARENT_DIR: &str = "..";
//type BoxedFileSystem = Box<dyn FileSystem<Inode = Inode, Handle = Handle> + Send + Sync>;
pub type BoxedLayer = Box<dyn Layer<Inode = Inode, Handle = Handle> + Send + Sync>;
const INODE_ALLOC_BATCH:u64 = 0x1_0000_0000;
// RealInode represents one inode object in specific layer.
// Also, each RealInode maps to one Entry, which should be 'forgotten' after drop.
// Important note: do not impl Clone trait for it or refcount will be messed up.
pub(crate) struct RealInode {
    pub layer: Arc<BoxedLayer>,
    pub in_upper_layer: bool,
    pub inode: u64,
    // File is whiteouted, we need to hide it.
    pub whiteout: bool,
    // Directory is opaque, we need to hide all entries inside it.
    pub opaque: bool,
    pub stat: Option<stat64>,
}

// OverlayInode must be protected by lock, it can be operated by multiple threads.
#[derive(Default)]
pub(crate) struct OverlayInode {
    // Inode hash table, map from 'name' to 'OverlayInode'.
    pub childrens: Mutex<HashMap<String, Arc<OverlayInode>>>,
    pub parent: Mutex<Weak<OverlayInode>>,
    // Backend inodes from all layers.
    pub real_inodes: Mutex<Vec<RealInode>>,
    // Inode number.
    pub inode: u64,
    pub path: String,
    pub name: String,
    pub lookups: AtomicU64,
    // Node is whiteout-ed.
    pub whiteout: AtomicBool,
    // Directory is loaded.
    pub loaded: AtomicBool,
}

#[derive(Default)]
#[allow(unused)]
pub enum CachePolicy {
    Never,
    #[default]
    Auto,
    Always,
}
pub struct OverlayFs {
    config: Config,
    lower_layers: Vec<Arc<BoxedLayer>>,
    upper_layer: Option<Arc<BoxedLayer>>,
    // All inodes in FS.
    inodes: RwLock<InodeStore>,
    // Open file handles.
    handles: Mutex<HashMap<u64, Arc<HandleData>>>,
    next_handle: AtomicU64,
    writeback: AtomicBool,
    no_open: AtomicBool,
    no_opendir: AtomicBool,
    killpriv_v2: AtomicBool,
    perfile_dax: AtomicBool,
    root_inodes: u64,
}

struct RealHandle {
    layer: Arc<BoxedLayer>,
    in_upper_layer: bool,
    inode: u64,
    handle: AtomicU64,
}

struct HandleData {
    node: Arc<OverlayInode>,
    //offset: libc::off_t,
    real_handle: Option<RealHandle>,
}

// RealInode is a wrapper of one inode in specific layer.
// All layer operations returning Entry should be wrapped in RealInode implementation
// so that we can increase the refcount(lookup count) of each inode and decrease it after Drop.
// Important: do not impl 'Copy' trait for it or refcount will be messed up.
impl RealInode {
    fn new(
        layer: Arc<BoxedLayer>,
        in_upper_layer: bool,
        inode: u64,
        whiteout: bool,
        opaque: bool,
    ) -> Self {
        let mut ri = RealInode {
            layer,
            in_upper_layer,
            inode,
            whiteout,
            opaque,
            stat: None,
        };
        match ri.stat64_ignore_enoent(&Context::default()) {
            Ok(v) => {
                ri.stat = v;
            }
            Err(e) => {
                error!("stat64 failed during RealInode creation: {}", e);
            }
        }
        ri
    }

    fn stat64(&self, ctx: &Context) -> Result<stat64> {
        let layer = self.layer.as_ref();
        if self.inode == 0 {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        match layer.getattr(ctx, self.inode, None) {
            Ok((v1, _v2)) => Ok(v1),
            Err(e) => Err(e),
        }
    }

    fn stat64_ignore_enoent(&self, ctx: &Context) -> Result<Option<stat64>> {
        match self.stat64(ctx) {
            Ok(v1) => Ok(Some(v1)),
            Err(e) => match e.raw_os_error() {
                Some(raw_error) => {
                    if raw_error != libc::ENOENT || raw_error != libc::ENAMETOOLONG {
                        return Ok(None);
                    }
                    Err(e)
                }
                None => Err(e),
            },
        }
    }

    // Do real lookup action in specific layer, this call will increase Entry refcount which must be released later.
    fn lookup_child_ignore_enoent(&self, ctx: &Context, name: &str) -> Result<Option<Entry>> {
        let cname = CString::new(name).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        // Real inode must have a layer.
        let layer = self.layer.as_ref();
        match layer.lookup(ctx, self.inode, cname.as_c_str()) {
            Ok(v) => {
                // Negative entry also indicates missing entry.
                if v.inode == 0 {
                    return Ok(None);
                }
                Ok(Some(v))
            }
            Err(e) => {
                if let Some(raw_error) = e.raw_os_error() {
                    if raw_error == libc::ENOENT || raw_error == libc::ENAMETOOLONG {
                        return Ok(None);
                    }
                }

                Err(e)
            }
        }
    }

    // Find child inode in same layer under this directory(Self).
    // Return None if not found.
    fn lookup_child(&self, ctx: &Context, name: &str) -> Result<Option<RealInode>> {
        if self.whiteout {
            return Ok(None);
        }

        let layer = self.layer.as_ref();

        // Find child Entry with <name> under directory with inode <self.inode>.
        match self.lookup_child_ignore_enoent(ctx, name)? {
            Some(v) => {
                // The Entry must be forgotten in each layer, which will be done automatically by Drop operation.
                let (whiteout, opaque) = if utils::is_dir(v.attr) {
                    (false, layer.is_opaque(ctx, v.inode)?)
                } else {
                    (layer.is_whiteout(ctx, v.inode)?, false)
                };

                Ok(Some(RealInode {
                    layer: self.layer.clone(),
                    in_upper_layer: self.in_upper_layer,
                    inode: v.inode,
                    whiteout,
                    opaque,
                    stat: Some(v.attr),
                }))
            }
            None => Ok(None),
        }
    }

    // Read directory entries from specific RealInode, error out if it's not directory.
    fn readdir(&self, ctx: &Context) -> Result<HashMap<String, RealInode>> {
        // Deleted inode should not be read.
        if self.whiteout {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        let stat = match self.stat {
            Some(v) => v,
            None => self.stat64(ctx)?,
        };

        // Must be directory.
        if !utils::is_dir(stat) {
            return Err(Error::from_raw_os_error(libc::ENOTDIR));
        }

        // Open the directory and load each entry.
        let opendir_res = self.layer.opendir(ctx, self.inode, libc::O_RDONLY as u32);
        let handle = match opendir_res {
            Ok((handle, _)) => handle.unwrap_or_default(),
   
            // opendir may not be supported if no_opendir is set, so we can ignore this error.
            Err(e) => {
                match e.raw_os_error() {
                    Some(raw_error) => {
                        if raw_error == libc::ENOSYS {
                            // We can still call readdir with inode if opendir is not supported in this layer.
                            0
                        } else {
                            return Err(e);
                        }
                    }
                    None => {
                        return Err(e);
                    }
                }
            }
        };

        let mut child_names = vec![];
        let mut more = true;
        let mut offset = 0;
        let bufsize = 1024;
        while more {
            more = false;
            self.layer.readdir(
                ctx,
                self.inode,
                handle,
                bufsize,
                offset,
                &mut |d| -> Result<usize> {
                    more = true;
                    offset = d.offset;
                    let child_name = String::from_utf8_lossy(d.name).into_owned();

                    trace!("entry: {}", child_name.as_str());

                    if child_name.eq(CURRENT_DIR) || child_name.eq(PARENT_DIR) {
                        return Ok(1);
                    }

                    child_names.push(child_name);

                    Ok(1)
                },
            )?;
        }

        // Non-zero handle indicates successful 'open', we should 'release' it.
        if handle > 0 {
            if let Err(e) = self
                .layer
                .releasedir(ctx, self.inode, libc::O_RDONLY as u32, handle)
            {
                // ignore ENOSYS
                match e.raw_os_error() {
                    Some(raw_error) => {
                        if raw_error != libc::ENOSYS {
                            return Err(e);
                        }
                    }
                    None => {
                        return Err(e);
                    }
                }
            }
        }

        // Lookup all child and construct "RealInode"s.
        let mut child_real_inodes = HashMap::new();
        for name in child_names {
            if let Some(child) = self.lookup_child(ctx, name.as_str())? {
                child_real_inodes.insert(name, child);
            }
        }

        Ok(child_real_inodes)
    }

    fn create_whiteout(&self, ctx: &Context, name: &str) -> Result<RealInode> {
        if !self.in_upper_layer {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        let cname = utils::to_cstring(name)?;
        let entry = self
            .layer
            .create_whiteout(ctx, self.inode, cname.as_c_str())?;

        // Wrap whiteout to RealInode.
        Ok(RealInode {
            layer: self.layer.clone(),
            in_upper_layer: true,
            inode: entry.inode,
            whiteout: true,
            opaque: false,
            stat: Some(entry.attr),
        })
    }

    fn mkdir(&self, ctx: &Context, name: &str, mode: u32, umask: u32) -> Result<RealInode> {
        if !self.in_upper_layer {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        let cname = utils::to_cstring(name)?;
        let entry = self
            .layer
            .mkdir(ctx, self.inode, cname.as_c_str(), mode, umask)?;

        // update node's first_layer
        Ok(RealInode {
            layer: self.layer.clone(),
            in_upper_layer: true,
            inode: entry.inode,
            whiteout: false,
            opaque: false,
            stat: Some(entry.attr),
        })
    }

    fn create(
        &self,
        ctx: &Context,
        name: &str,
        args: CreateIn,
    ) -> Result<(RealInode, Option<u64>)> {
        if !self.in_upper_layer {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        let (entry, h, _, _) =
            self.layer
                .create(ctx, self.inode, utils::to_cstring(name)?.as_c_str(), args)?;

        Ok((
            RealInode {
                layer: self.layer.clone(),
                in_upper_layer: true,
                inode: entry.inode,
                whiteout: false,
                opaque: false,
                stat: Some(entry.attr),
            },
            h,
        ))
    }

    fn mknod(
        &self,
        ctx: &Context,
        name: &str,
        mode: u32,
        rdev: u32,
        umask: u32,
    ) -> Result<RealInode> {
        if !self.in_upper_layer {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        let entry = self.layer.mknod(
            ctx,
            self.inode,
            utils::to_cstring(name)?.as_c_str(),
            mode,
            rdev,
            umask,
        )?;
        Ok(RealInode {
            layer: self.layer.clone(),
            in_upper_layer: true,
            inode: entry.inode,
            whiteout: false,
            opaque: false,
            stat: Some(entry.attr),
        })
    }

    fn link(&self, ctx: &Context, ino: u64, name: &str) -> Result<RealInode> {
        if !self.in_upper_layer {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        let entry = self
            .layer
            .link(ctx, ino, self.inode, utils::to_cstring(name)?.as_c_str())?;

        let opaque = if utils::is_dir(entry.attr) {
            self.layer.is_opaque(ctx, entry.inode)?
        } else {
            false
        };
        Ok(RealInode {
            layer: self.layer.clone(),
            in_upper_layer: true,
            inode: entry.inode,
            whiteout: false,
            opaque,
            stat: Some(entry.attr),
        })
    }

    // Create a symlink in self directory.
    fn symlink(&self, ctx: &Context, link_name: &str, filename: &str) -> Result<RealInode> {
        if !self.in_upper_layer {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        let entry = self.layer.symlink(
            ctx,
            utils::to_cstring(link_name)?.as_c_str(),
            self.inode,
            utils::to_cstring(filename)?.as_c_str(),
        )?;

        Ok(RealInode {
            layer: self.layer.clone(),
            in_upper_layer: self.in_upper_layer,
            inode: entry.inode,
            whiteout: false,
            opaque: false,
            stat: Some(entry.attr),
        })
    }
}

impl Drop for RealInode {
    fn drop(&mut self) {
        // Release refcount of inode in layer.
        let ctx = Context::default();
        let layer = self.layer.as_ref();
        let inode = self.inode;
        debug!("forget inode {} by 1 for backend inode in layer ", inode);
        layer.forget(&ctx, inode, 1);
    }
}

impl OverlayInode {
    pub fn new() -> Self {
        OverlayInode::default()
    }
    // Allocate new OverlayInode based on one RealInode,
    // inode number is always 0 since only OverlayFs has global unique inode allocator.
    pub fn new_from_real_inode(name: &str, ino: u64, path: String, real_inode: RealInode) -> Self {
        let mut new = OverlayInode::new();
        new.inode = ino;
        new.path = path;
        new.name = name.to_string();
        new.whiteout.store(real_inode.whiteout, Ordering::Relaxed);
        new.lookups = AtomicU64::new(1);
        new.real_inodes = Mutex::new(vec![real_inode]);
        new
    }

    pub fn new_from_real_inodes(
        name: &str,
        ino: u64,
        path: String,
        real_inodes: Vec<RealInode>,
    ) -> Result<Self> {
        if real_inodes.is_empty() {
            error!("BUG: new_from_real_inodes() called with empty real_inodes");
            return Err(Error::from_raw_os_error(libc::EINVAL));
        }

        let mut first = true;
        let mut new = Self::new();
        for ri in real_inodes {
            let whiteout = ri.whiteout;
            let opaque = ri.opaque;
            let stat = match ri.stat {
                Some(v) => v,
                None => ri.stat64(&Context::default())?,
            };

            if first {
                first = false;
                new = Self::new_from_real_inode(name, ino, path.clone(), ri);

                // This is whiteout, no need to check lower layers.
                if whiteout {
                    break;
                }

                // A non-directory file shadows all lower layers as default.
                if !utils::is_dir(stat) {
                    break;
                }

                // Opaque directory shadows all lower layers.
                if opaque {
                    break;
                }
            } else {
                // This is whiteout, no need to record this, break directly.
                if ri.whiteout {
                    break;
                }

                // Only directory have multiple real inodes, so if this is non-first real-inode
                // and it's not directory, it should indicates some invalid layout. @weizhang555
                if !utils::is_dir(stat) {
                    error!("invalid layout: non-directory has multiple real inodes");
                    break;
                }

                // Valid directory.
                new.real_inodes.lock().unwrap().push(ri);
                // Opaque directory shadows all lower layers.
                if opaque {
                    break;
                }
            }
        }
        Ok(new)
    }

    pub fn stat64(&self, ctx: &Context) -> Result<stat64> {
        // try layers in order or just take stat from first layer?
        for l in self.real_inodes.lock().unwrap().iter() {
            if let Some(v) = l.stat64_ignore_enoent(ctx)? {
                return Ok(v);
            }
        }

        // not in any layer
        Err(Error::from_raw_os_error(libc::ENOENT))
    }

    pub fn count_entries_and_whiteout(&self, ctx: &Context) -> Result<(u64, u64)> {
        let mut count = 0;
        let mut whiteouts = 0;

        let st = self.stat64(ctx)?;

        // must be directory
        if !utils::is_dir(st) {
            return Err(Error::from_raw_os_error(libc::ENOTDIR));
        }

        for (_, child) in self.childrens.lock().unwrap().iter() {
            if child.whiteout.load(Ordering::Relaxed) {
                whiteouts += 1;
            } else {
                count += 1;
            }
        }

        Ok((count, whiteouts))
    }

    pub fn open(
        &self,
        ctx: &Context,
        flags: u32,
        fuse_flags: u32,
    ) -> Result<(Arc<BoxedLayer>, Option<Handle>, OpenOptions)> {
        let (layer, _, inode) = self.first_layer_inode();
        let (h, o, _) = layer.as_ref().open(ctx, inode, flags, fuse_flags)?;
        Ok((layer, h, o))
    }

    // Self is directory, fill all childrens.
    pub fn scan_childrens(self: &Arc<Self>, ctx: &Context) -> Result<Vec<OverlayInode>> {
        let st = self.stat64(ctx)?;
        if !utils::is_dir(st) {
            return Err(Error::from_raw_os_error(libc::ENOTDIR));
        }

        let mut all_layer_inodes: HashMap<String, Vec<RealInode>> = HashMap::new();
        // read out directories from each layer
        let mut counter = 1;
        let layers_count = self.real_inodes.lock().unwrap().len();
        // Scan from upper layer to lower layer.
        for ri in self.real_inodes.lock().unwrap().iter() {
            debug!(
                "loading Layer {}/{} for dir '{}', is_upper_layer: {}",
                counter,
                layers_count,
                self.path.as_str(),
                ri.in_upper_layer
            );
            counter += 1;
            if ri.whiteout {
                // Node is deleted from some upper layer, skip it.
                debug!("directory is whiteout");
                break;
            }

            let stat = match ri.stat {
                Some(v) => v,
                None => ri.stat64(ctx)?,
            };

            if !utils::is_dir(stat) {
                debug!("{} is not a directory", self.path.as_str());
                // not directory
                break;
            }

            // Read all entries from one layer.
            let entries = ri.readdir(ctx)?;

            // Merge entries from one layer to all_layer_inodes.
            for (name, inode) in entries {
                match all_layer_inodes.get_mut(&name) {
                    Some(v) => {
                        // Append additional RealInode to the end of vector.
                        v.push(inode)
                    }
                    None => {
                        all_layer_inodes.insert(name, vec![inode]);
                    }
                };
            }

            // if opaque, stop here
            if ri.opaque {
                debug!("directory {} is opaque", self.path.as_str());
                break;
            }
        }

        // Construct OverlayInode for each entry.
        let mut childrens = vec![];
        for (name, real_inodes) in all_layer_inodes {
            // Inode numbers are not allocated yet.
            let path = format!("{}/{}", self.path, name);
            let new = Self::new_from_real_inodes(name.as_str(), 0, path, real_inodes)?;
            childrens.push(new);
        }

        Ok(childrens)
    }

    // Create a new directory in upper layer for node, node must be directory.
    pub fn create_upper_dir(
        self: &Arc<Self>,
        ctx: &Context,
        mode_umask: Option<(u32, u32)>,
    ) -> Result<()> {
        let st = self.stat64(ctx)?;
        if !utils::is_dir(st) {
            return Err(Error::from_raw_os_error(libc::ENOTDIR));
        }

        // If node already has upper layer, we can just return here.
        if self.in_upper_layer() {
            return Ok(());
        }

        // not in upper layer, check parent.
        let pnode = if let Some(n) = self.parent.lock().unwrap().upgrade() {
            Arc::clone(&n)
        } else {
            return Err(Error::new(ErrorKind::Other, "no parent?"));
        };

        if !pnode.in_upper_layer() {
            pnode.create_upper_dir(ctx, None)?; // recursive call
        }
        let mut child = None;
        pnode.handle_upper_inode_locked(&mut |parent_upper_inode| -> Result<bool> {
            match parent_upper_inode {
                Some(parent_ri) => {
                    let ri = match mode_umask {
                        Some((mode, umask)) => {
                            parent_ri.mkdir(ctx, self.name.as_str(), mode, umask)?
                        }
                        None => parent_ri.mkdir(ctx, self.name.as_str(), st.st_mode, 0)?,
                    };
                    // create directory here
                    child.replace(ri);
                }
                None => {
                    error!(
                        "BUG: parent {} has no upper inode after create_upper_dir",
                        pnode.inode
                    );
                    return Err(Error::from_raw_os_error(libc::EINVAL));
                }
            }
            Ok(false)
        })?;

        if let Some(ri) = child {
            // Push the new real inode to the front of vector.
            self.add_upper_inode(ri, false);
        }

        Ok(())
    }

    // Add new upper RealInode to OverlayInode, clear all lower RealInodes if 'clear_lowers' is true.
    fn add_upper_inode(self: &Arc<Self>, ri: RealInode, clear_lowers: bool) {
        let mut inodes = self.real_inodes.lock().unwrap();
        // Update self according to upper attribute.
        self.whiteout.store(ri.whiteout, Ordering::Relaxed);

        // Push the new real inode to the front of vector.
        let mut new = vec![ri];
        // Drain lower RealInodes.
        let lowers = inodes.drain(..).collect::<Vec<RealInode>>();
        if !clear_lowers {
            // If not clear lowers, append them to the end of vector.
            new.extend(lowers);
        }
        inodes.extend(new);
    }

    pub fn in_upper_layer(&self) -> bool {
        let all_inodes = self.real_inodes.lock().unwrap();
        let first = all_inodes.first();
        match first {
            Some(v) => v.in_upper_layer,
            None => false,
        }
    }

    pub fn upper_layer_only(&self) -> bool {
        let real_inodes = self.real_inodes.lock().unwrap();
        let first = real_inodes.first();
        match first {
            Some(v) => {
                if !v.in_upper_layer {
                    false
                } else {
                    real_inodes.len() == 1
                }
            }
            None => false,
        }
    }

    pub fn first_layer_inode(&self) -> (Arc<BoxedLayer>, bool, u64) {
        let all_inodes = self.real_inodes.lock().unwrap();
        let first = all_inodes.first();
        match first {
            Some(v) => (v.layer.clone(), v.in_upper_layer, v.inode),
            None => panic!("BUG: dangling OverlayInode"),
        }
    }

    pub fn child(&self, name: &str) -> Option<Arc<OverlayInode>> {
        self.childrens.lock().unwrap().get(name).cloned()
    }

    pub fn remove_child(&self, name: &str) {
        self.childrens.lock().unwrap().remove(name);
    }

    pub fn insert_child(&self, name: &str, node: Arc<OverlayInode>) {
        self.childrens
            .lock()
            .unwrap()
            .insert(name.to_string(), node);
    }

    pub fn handle_upper_inode_locked(
        &self,
        f: &mut dyn FnMut(Option<&RealInode>) -> Result<bool>,
    ) -> Result<bool> {
        let all_inodes = self.real_inodes.lock().unwrap();
        let first = all_inodes.first();
        match first {
            Some(v) => {
                if v.in_upper_layer {
                    f(Some(v))
                } else {
                    f(None)
                }
            }
            None => Err(Error::new(
                ErrorKind::Other,
                format!(
                    "BUG: dangling OverlayInode {} without any backend inode",
                    self.inode
                ),
            )),
        }
    }
}

fn entry_type_from_mode(mode: libc::mode_t) -> u8 {
    match mode & libc::S_IFMT {
        libc::S_IFBLK => libc::DT_BLK,
        libc::S_IFCHR => libc::DT_CHR,
        libc::S_IFDIR => libc::DT_DIR,
        libc::S_IFIFO => libc::DT_FIFO,
        libc::S_IFLNK => libc::DT_LNK,
        libc::S_IFREG => libc::DT_REG,
        libc::S_IFSOCK => libc::DT_SOCK,
        _ => libc::DT_UNKNOWN,
    }
}
#[allow(unused)]
impl OverlayFs {
    pub fn new(
        upper: Option<Arc<BoxedLayer>>,
        lowers: Vec<Arc<BoxedLayer>>,
        params: Config,
        root_inode:u64,
    ) -> Result<Self> {
        // load root inode
        Ok(OverlayFs {
            config: params,
            lower_layers: lowers,
            upper_layer: upper,
            inodes: RwLock::new(InodeStore::new()),
            handles: Mutex::new(HashMap::new()),
            next_handle: AtomicU64::new(1),
            writeback: AtomicBool::new(false),
            no_open: AtomicBool::new(false),
            no_opendir: AtomicBool::new(false),
            killpriv_v2: AtomicBool::new(false),
            perfile_dax: AtomicBool::new(false),
            root_inodes: root_inode,
        })
    }

    pub fn root_inode(&self) -> Inode {
        self.root_inodes
    }

    fn alloc_inode(&self, path: &String) -> Result<u64> {
        self.inodes.write().unwrap().alloc_inode(path)
    }

    pub fn import(&self) -> Result<()> {
        let mut root = OverlayInode::new();
        root.inode = self.root_inode();
        root.path = String::from("");
        root.name = String::from("");
        root.lookups = AtomicU64::new(2);
        root.real_inodes = Mutex::new(vec![]);
        let ctx = Context::default();

        // Update upper inode
        if let Some(layer) = self.upper_layer.as_ref() {
            let ino = layer.root_inode();
            let real = RealInode::new(
                layer.clone(), 
                true, ino, 
                false, 
                layer.is_opaque(&ctx, ino)?
            );
            root.real_inodes.lock().unwrap().push(real);
        }

        // Update lower inodes.
        for layer in self.lower_layers.iter() {
            let ino = layer.root_inode();
            let real: RealInode = RealInode::new(
                layer.clone(),
                false,
                ino,
                false,
                layer.is_opaque(&ctx, ino)?,
            );
            root.real_inodes.lock().unwrap().push(real);
        }
        let root_node = Arc::new(root);

        // insert root inode into hash
        self.insert_inode(self.root_inode(), Arc::clone(&root_node));

        info!("loading root directory\n");
        self.load_directory(&ctx, &root_node)?;

        Ok(())
    }

    fn root_node(&self) -> Arc<OverlayInode> {
        // Root node must exist.
        self.get_active_inode(self.root_inode()).unwrap()
    }

    fn insert_inode(&self, inode: u64, node: Arc<OverlayInode>) {
        self.inodes.write().unwrap().insert_inode(inode, node);
    }

    fn get_active_inode(&self, inode: u64) -> Option<Arc<OverlayInode>> {
        self.inodes.read().unwrap().get_inode(inode)
    }

    // Get inode which is active or deleted.
    fn get_all_inode(&self, inode: u64) -> Option<Arc<OverlayInode>> {
        let inode_store = self.inodes.read().unwrap();
        match inode_store.get_inode(inode) {
            Some(n) => Some(n),
            None => inode_store.get_deleted_inode(inode),
        }
    }

    // Return the inode only if it's permanently deleted from both self.inodes and self.deleted_inodes.
    fn remove_inode(&self, inode: u64, path_removed: Option<String>) -> Option<Arc<OverlayInode>> {
        self.inodes
            .write()
            .unwrap()
            .remove_inode(inode, path_removed)
    }

    // Lookup child OverlayInode with <name> under <parent> directory.
    // If name is empty, return parent itself.
    // Parent dir will be loaded, but returned OverlayInode won't.
    fn lookup_node(&self, ctx: &Context, parent: Inode, name: &str) -> Result<Arc<OverlayInode>> {
        if name.contains([SLASH_ASCII as char]) {
            return Err(Error::from_raw_os_error(libc::EINVAL));
        }

        // Parent inode is expected to be loaded before this function is called.
        let pnode = match self.get_active_inode(parent) {
            Some(v) => v,
            None => return Err(Error::from_raw_os_error(libc::ENOENT)),
        };

        // Parent is whiteout-ed, return ENOENT.
        if pnode.whiteout.load(Ordering::Relaxed) {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        let st = pnode.stat64(ctx)?;
        if utils::is_dir(st) && !pnode.loaded.load(Ordering::Relaxed) {
            // Parent is expected to be directory, load it first.
            self.load_directory(ctx, &pnode)?;
        }

        // Current file or dir.
        if name.eq(".")  
            // Root directory has no parent.
            || (parent == self.root_inode() && name.eq("..")) 
            // Special convention: empty name indicates current dir.
            || name.is_empty()
        {
            return Ok(Arc::clone(&pnode));
        }

        match pnode.child(name) {
            // Child is found.
            Some(v) => Ok(v),
            None => Err(Error::from_raw_os_error(libc::ENOENT)),
        }
    }

    // As a debug function, print all inode numbers in hash table.
    #[allow(dead_code)]
    fn debug_print_all_inodes(&self) {
        self.inodes.read().unwrap().debug_print_all_inodes();
    }

    fn lookup_node_ignore_enoent(
        &self,
        ctx: &Context,
        parent: u64,
        name: &str,
    ) -> Result<Option<Arc<OverlayInode>>> {
        match self.lookup_node(ctx, parent, name) {
            Ok(n) => Ok(Some(Arc::clone(&n))),
            Err(e) => {
                if let Some(raw_error) = e.raw_os_error() {
                    if raw_error == libc::ENOENT {
                        return Ok(None);
                    }
                }
                Err(e)
            }
        }
    }

    // Load entries of the directory from all layers, if node is not directory, return directly.
    fn load_directory(&self, ctx: &Context, node: &Arc<OverlayInode>) -> Result<()> {
        if node.loaded.load(Ordering::Relaxed) {
            return Ok(());
        }

        // We got all childrens without inode.
        let childrens = node.scan_childrens(ctx)?;

        // =============== Start Lock Area ===================
        // Lock OverlayFs inodes.
        let mut inode_store = self.inodes.write().unwrap();
        // Lock the OverlayInode and its childrens.
        let mut node_children = node.childrens.lock().unwrap();

        // Check again in case another 'load_directory' function call gets locks and want to do duplicated work.
        if node.loaded.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Now we have two locks' protection, Fs inodes lock and OverlayInode's childrens lock.
        for mut child in childrens.into_iter() {
            // Allocate inode for each child.
            let ino = inode_store.alloc_inode(&child.path)?;

            let name = child.name.clone();
            child.inode = ino;
            // Create bi-directional link between parent and child.
            child.parent = Mutex::new(Arc::downgrade(node));

            let arc_child = Arc::new(child);
            node_children.insert(name, arc_child.clone());
            // Record overlay inode in whole OverlayFs.
            inode_store.insert_inode(ino, arc_child.clone());
        }

        node.loaded.store(true, Ordering::Relaxed);

        Ok(())
    }

    fn forget_one(&self, inode: Inode, count: u64) {
        if inode == self.root_inode() || inode == 0 {
            return;
        }

        let v = match self.get_all_inode(inode) {
            Some(n) => n,
            None => {
                trace!("forget unknown inode: {}", inode);
                return;
            }
        };

        // FIXME: need atomic protection around lookups' load & store. @weizhang555
        let mut lookups = v.lookups.load(Ordering::Relaxed);

        if lookups < count {
            lookups = 0;
        } else {
            lookups -= count;
        }
        v.lookups.store(lookups, Ordering::Relaxed);

        // TODO: use compare_exchange.
        //v.lookups.compare_exchange(old, new, Ordering::Acquire, Ordering::Relaxed);

        if lookups == 0 {
            debug!("inode is forgotten: {}, name {}", inode, v.name);
            let _ = self.remove_inode(inode, None);
            let parent = v.parent.lock().unwrap();

            if let Some(p) = parent.upgrade() {
                // remove it from hashmap
                p.remove_child(v.name.as_str());
            }
        }
    }

    fn do_lookup(&self, ctx: &Context, parent: Inode, name: &str) -> Result<Entry> {
        let node = self.lookup_node(ctx, parent, name)?;

        if node.whiteout.load(Ordering::Relaxed) {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        let st = node.stat64(ctx)?;

        if utils::is_dir(st) && !node.loaded.load(Ordering::Relaxed) {
            self.load_directory(ctx, &node)?;
        }

        // FIXME: can forget happen between found and increase reference counter?
        let tmp = node.lookups.fetch_add(1, Ordering::Relaxed);
        trace!("lookup count: {}", tmp + 1);
        Ok(Entry {
            inode: node.inode,
            generation: 0,
            attr: st,
            attr_flags: 0,
            attr_timeout: self.config.attr_timeout,
            entry_timeout: self.config.entry_timeout,
        })
    }

    fn do_statvfs(&self, ctx: &Context, inode: Inode) -> Result<statvfs64> {
        match self.get_active_inode(inode) {
            Some(ovi) => {
                let all_inodes = ovi.real_inodes.lock().unwrap();
                let real_inode = all_inodes
                    .first()
                    .ok_or(Error::new(ErrorKind::Other, "backend inode not found"))?;
                real_inode.layer.statfs(ctx, real_inode.inode)
            }
            None => Err(Error::from_raw_os_error(libc::ENOENT)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn do_readdir(
        &self,
        ctx: &Context,
        inode: Inode,
        handle: u64,
        size: u32,
        offset: u64,
        is_readdirplus: bool,
        add_entry: &mut dyn FnMut(DirEntry, Option<Entry>) -> Result<usize>,
    ) -> Result<()> {
        trace!(
            "do_readir: handle: {}, size: {}, offset: {}",
            handle,
            size,
            offset
        );
        if size == 0 {
            return Ok(());
        }

        // lookup the directory
        let ovl_inode = match self.handles.lock().unwrap().get(&handle) {
            Some(dir) => dir.node.clone(),
            None => {
                // Try to get data with inode.
                let node = self.lookup_node(ctx, inode, ".")?;

                let st = node.stat64(ctx)?;
                if !utils::is_dir(st) {
                    return Err(Error::from_raw_os_error(libc::ENOTDIR));
                }

                node.clone()
            }
        };

        let mut childrens = Vec::new();
        //add myself as "."
        childrens.push((".".to_string(), ovl_inode.clone()));

        //add parent
        let parent_node = match ovl_inode.parent.lock().unwrap().upgrade() {
            Some(p) => p.clone(),
            None => self.root_node(),
        };
        childrens.push(("..".to_string(), parent_node));

        for (_, child) in ovl_inode.childrens.lock().unwrap().iter() {
            // skip whiteout node
            if child.whiteout.load(Ordering::Relaxed) {
                continue;
            }
            childrens.push((child.name.clone(), child.clone()));
        }

        let mut len: usize = 0;
        if offset >= childrens.len() as u64 {
            return Ok(());
        }

        for (index, (name, child)) in (0_u64..).zip(childrens.into_iter()) {
            if index >= offset {
                // make struct DireEntry and Entry
                let st = child.stat64(ctx)?;
                let dir_entry = DirEntry {
                    ino: st.st_ino,
                    offset: index + 1,
                    type_: entry_type_from_mode(st.st_mode) as u32,
                    name: name.as_bytes(),
                };

                let entry = if is_readdirplus {
                    child.lookups.fetch_add(1, Ordering::Relaxed);
                    Some(Entry {
                        inode: child.inode,
                        generation: 0,
                        attr: st,
                        attr_flags: 0,
                        attr_timeout: self.config.attr_timeout,
                        entry_timeout: self.config.entry_timeout,
                    })
                } else {
                    None
                };
                match add_entry(dir_entry, entry) {
                    Ok(0) => break,
                    Ok(l) => {
                        len += l;
                        if len as u32 >= size {
                            // no more space, stop here
                            return Ok(());
                        }
                    }

                    Err(e) => {
                        // when the buffer is still empty, return error, otherwise return the entry already added
                        if len == 0 {
                            return Err(e);
                        } else {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn do_mkdir(
        &self,
        ctx: &Context,
        parent_node: &Arc<OverlayInode>,
        name: &str,
        mode: u32,
        umask: u32,
    ) -> Result<()> {
        if self.upper_layer.is_none() {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        // Parent node was deleted.
        if parent_node.whiteout.load(Ordering::Relaxed) {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        let mut delete_whiteout = false;
        let mut set_opaque = false;
        if let Some(n) = self.lookup_node_ignore_enoent(ctx, parent_node.inode, name)? {
            // Node with same name exists, let's check if it's whiteout.
            if !n.whiteout.load(Ordering::Relaxed) {
                return Err(Error::from_raw_os_error(libc::EEXIST));
            }

            if n.in_upper_layer() {
                delete_whiteout = true;
            }

            // Set opaque if child dir has lower layers.
            if !n.upper_layer_only() {
                set_opaque = true;
            }
        }

        // Copy parent node up if necessary.
        let pnode = self.copy_node_up(ctx, Arc::clone(parent_node))?;

        let mut new_node = None;
        let path = format!("{}/{}", pnode.path, name);
        pnode.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
            let parent_real_inode = match parent_real_inode {
                Some(inode) => inode,
                None => {
                    error!("BUG: parent doesn't have upper inode after copied up");
                    return Err(Error::from_raw_os_error(libc::EINVAL));
                }
            };

            if delete_whiteout {
                let _ = parent_real_inode.layer.delete_whiteout(
                    ctx,
                    parent_real_inode.inode,
                    utils::to_cstring(name)?.as_c_str(),
                );
            }
            // Allocate inode number.
            let ino = self.alloc_inode(&path)?;
            let child_dir = parent_real_inode.mkdir(ctx, name, mode, umask)?;
            // Set opaque if child dir has lower layers.
            if set_opaque {
                parent_real_inode.layer.set_opaque(ctx, child_dir.inode)?;
            }
            let ovi = OverlayInode::new_from_real_inode(name, ino, path.clone(), child_dir);

            new_node.replace(ovi);
            Ok(false)
        })?;

        // new_node is always 'Some'
        let arc_node = Arc::new(new_node.unwrap());
        self.insert_inode(arc_node.inode, arc_node.clone());
        pnode.insert_child(name, arc_node);
        Ok(())
    }

    fn do_mknod(
        &self,
        ctx: &Context,
        parent_node: &Arc<OverlayInode>,
        name: &str,
        mode: u32,
        rdev: u32,
        umask: u32,
    ) -> Result<()> {
        if self.upper_layer.is_none() {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        // Parent node was deleted.
        if parent_node.whiteout.load(Ordering::Relaxed) {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        match self.lookup_node_ignore_enoent(ctx, parent_node.inode, name)? {
            Some(n) => {
                // Node with same name exists, let's check if it's whiteout.
                if !n.whiteout.load(Ordering::Relaxed) {
                    return Err(Error::from_raw_os_error(libc::EEXIST));
                }

                // Copy parent node up if necessary.
                let pnode = self.copy_node_up(ctx, Arc::clone(parent_node))?;
                pnode.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    if n.in_upper_layer() {
                        let _ = parent_real_inode.layer.delete_whiteout(
                            ctx,
                            parent_real_inode.inode,
                            utils::to_cstring(name)?.as_c_str(),
                        );
                    }

                    let child_ri = parent_real_inode.mknod(ctx, name, mode, rdev, umask)?;

                    // Replace existing real inodes with new one.
                    n.add_upper_inode(child_ri, true);
                    Ok(false)
                })?;
            }
            None => {
                // Copy parent node up if necessary.
                let pnode = self.copy_node_up(ctx, Arc::clone(parent_node))?;
                let mut new_node = None;
                let path = format!("{}/{}", pnode.path, name);
                pnode.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    // Allocate inode number.
                    let ino = self.alloc_inode(&path)?;
                    let child_ri = parent_real_inode.mknod(ctx, name, mode, rdev, umask)?;
                    let ovi = OverlayInode::new_from_real_inode(name, ino, path.clone(), child_ri);

                    new_node.replace(ovi);
                    Ok(false)
                })?;

                // new_node is always 'Some'
                let arc_node = Arc::new(new_node.unwrap());
                self.insert_inode(arc_node.inode, arc_node.clone());
                pnode.insert_child(name, arc_node);
            }
        }

        Ok(())
    }

    fn do_create(
        &self,
        ctx: &Context,
        parent_node: &Arc<OverlayInode>,
        name: &str,
        args: CreateIn,
    ) -> Result<Option<u64>> {
        let upper = self
            .upper_layer
            .as_ref()
            .cloned()
            .ok_or_else(|| Error::from_raw_os_error(libc::EROFS))?;

        // Parent node was deleted.
        if parent_node.whiteout.load(Ordering::Relaxed) {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        let mut handle = None;
        let mut real_ino = 0u64;
        let new_ovi = match self.lookup_node_ignore_enoent(ctx, parent_node.inode, name)? {
            Some(n) => {
                // Node with same name exists, let's check if it's whiteout.
                if !n.whiteout.load(Ordering::Relaxed) {
                    return Err(Error::from_raw_os_error(libc::EEXIST));
                }

                // Copy parent node up if necessary.
                let pnode = self.copy_node_up(ctx, Arc::clone(parent_node))?;
                pnode.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    if n.in_upper_layer() {
                        let _ = parent_real_inode.layer.delete_whiteout(
                            ctx,
                            parent_real_inode.inode,
                            utils::to_cstring(name)?.as_c_str(),
                        );
                    }

                    let (child_ri, hd) = parent_real_inode.create(ctx, name, args)?;
                    real_ino = child_ri.inode;
                    handle = hd;

                    // Replace existing real inodes with new one.
                    n.add_upper_inode(child_ri, true);
                    Ok(false)
                })?;
                n.clone()
            }
            None => {
                // Copy parent node up if necessary.
                let pnode = self.copy_node_up(ctx, Arc::clone(parent_node))?;
                let mut new_node = None;
                let path = format!("{}/{}", pnode.path, name);
                pnode.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    let (child_ri, hd) = parent_real_inode.create(ctx, name, args)?;
                    real_ino = child_ri.inode;
                    handle = hd;
                    // Allocate inode number.
                    let ino = self.alloc_inode(&path)?;
                    let ovi = OverlayInode::new_from_real_inode(name, ino, path.clone(), child_ri);

                    new_node.replace(ovi);
                    Ok(false)
                })?;

                // new_node is always 'Some'
                let arc_node = Arc::new(new_node.unwrap());
                self.insert_inode(arc_node.inode, arc_node.clone());
                pnode.insert_child(name, arc_node.clone());
                arc_node
            }
        };

        let final_handle = match handle {
            Some(hd) => {
                if self.no_open.load(Ordering::Relaxed) {
                    None
                } else {
                    let handle = self.next_handle.fetch_add(1, Ordering::Relaxed);
                    let handle_data = HandleData {
                        node: new_ovi,
                        real_handle: Some(RealHandle {
                            layer: upper.clone(),
                            in_upper_layer: true,
                            inode: real_ino,
                            handle: AtomicU64::new(hd),
                        }),
                    };
                    self.handles
                        .lock()
                        .unwrap()
                        .insert(handle, Arc::new(handle_data));
                    Some(handle)
                }
            }
            None => None,
        };
        Ok(final_handle)
    }

    fn do_link(
        &self,
        ctx: &Context,
        src_node: &Arc<OverlayInode>,
        new_parent: &Arc<OverlayInode>,
        name: &str,
    ) -> Result<()> {
        if self.upper_layer.is_none() {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        // Node is whiteout.
        if src_node.whiteout.load(Ordering::Relaxed) || new_parent.whiteout.load(Ordering::Relaxed)
        {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        let st = src_node.stat64(ctx)?;
        if utils::is_dir(st) {
            // Directory can't be hardlinked.
            return Err(Error::from_raw_os_error(libc::EPERM));
        }

        let src_node = self.copy_node_up(ctx, Arc::clone(src_node))?;
        let new_parent = self.copy_node_up(ctx, Arc::clone(new_parent))?;
        let src_ino = src_node.first_layer_inode().2;

        match self.lookup_node_ignore_enoent(ctx, new_parent.inode, name)? {
            Some(n) => {
                // Node with same name exists, let's check if it's whiteout.
                if !n.whiteout.load(Ordering::Relaxed) {
                    return Err(Error::from_raw_os_error(libc::EEXIST));
                }

                // Node is definitely a whiteout now.
                new_parent.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    // Whiteout file exists in upper level, let's delete it.
                    if n.in_upper_layer() {
                        let _ = parent_real_inode.layer.delete_whiteout(
                            ctx,
                            parent_real_inode.inode,
                            utils::to_cstring(name)?.as_c_str(),
                        );
                    }

                    let child_ri = parent_real_inode.link(ctx, src_ino, name)?;

                    // Replace existing real inodes with new one.
                    n.add_upper_inode(child_ri, true);
                    Ok(false)
                })?;
            }
            None => {
                // Copy parent node up if necessary.
                let mut new_node = None;
                new_parent.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    // Allocate inode number.
                    let path = format!("{}/{}", new_parent.path, name);
                    let ino = self.alloc_inode(&path)?;
                    let child_ri = parent_real_inode.link(ctx, src_ino, name)?;
                    let ovi = OverlayInode::new_from_real_inode(name, ino, path, child_ri);

                    new_node.replace(ovi);
                    Ok(false)
                })?;

                // new_node is always 'Some'
                let arc_node = Arc::new(new_node.unwrap());
                self.insert_inode(arc_node.inode, arc_node.clone());
                new_parent.insert_child(name, arc_node);
            }
        }

        Ok(())
    }

    fn do_symlink(
        &self,
        ctx: &Context,
        linkname: &str,
        parent_node: &Arc<OverlayInode>,
        name: &str,
    ) -> Result<()> {
        if self.upper_layer.is_none() {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        // parent was deleted.
        if parent_node.whiteout.load(Ordering::Relaxed) {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        match self.lookup_node_ignore_enoent(ctx, parent_node.inode, name)? {
            Some(n) => {
                // Node with same name exists, let's check if it's whiteout.
                if !n.whiteout.load(Ordering::Relaxed) {
                    return Err(Error::from_raw_os_error(libc::EEXIST));
                }

                // Copy parent node up if necessary.
                let pnode = self.copy_node_up(ctx, Arc::clone(parent_node))?;
                pnode.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    if n.in_upper_layer() {
                        let _ = parent_real_inode.layer.delete_whiteout(
                            ctx,
                            parent_real_inode.inode,
                            utils::to_cstring(name)?.as_c_str(),
                        );
                    }

                    let child_ri = parent_real_inode.symlink(ctx, linkname, name)?;

                    // Replace existing real inodes with new one.
                    n.add_upper_inode(child_ri, true);
                    Ok(false)
                })?;
            }
            None => {
                // Copy parent node up if necessary.
                let pnode = self.copy_node_up(ctx, Arc::clone(parent_node))?;
                let mut new_node = None;
                let path = format!("{}/{}", pnode.path, name);
                pnode.handle_upper_inode_locked(&mut |parent_real_inode| -> Result<bool> {
                    let parent_real_inode = match parent_real_inode {
                        Some(inode) => inode,
                        None => {
                            error!("BUG: parent doesn't have upper inode after copied up");
                            return Err(Error::from_raw_os_error(libc::EINVAL));
                        }
                    };

                    // Allocate inode number.
                    let ino = self.alloc_inode(&path)?;
                    let child_ri = parent_real_inode.symlink(ctx, linkname, name)?;
                    let ovi = OverlayInode::new_from_real_inode(name, ino, path.clone(), child_ri);

                    new_node.replace(ovi);
                    Ok(false)
                })?;

                // new_node is always 'Some'
                let arc_node = Arc::new(new_node.unwrap());
                self.insert_inode(arc_node.inode, arc_node.clone());
                pnode.insert_child(name, arc_node);
            }
        }

        Ok(())
    }

    fn copy_symlink_up(&self, ctx: &Context, node: Arc<OverlayInode>) -> Result<Arc<OverlayInode>> {
        if node.in_upper_layer() {
            return Ok(node);
        }

        let parent_node = if let Some(ref n) = node.parent.lock().unwrap().upgrade() {
            Arc::clone(n)
        } else {
            return Err(Error::new(ErrorKind::Other, "no parent?"));
        };

        let (self_layer, _, self_inode) = node.first_layer_inode();

        if !parent_node.in_upper_layer() {
            parent_node.create_upper_dir(ctx, None)?;
        }

        // Read the linkname from lower layer.
        let path = self_layer.readlink(ctx, self_inode)?;
        // Convert path to &str.
        let path =
            std::str::from_utf8(&path).map_err(|_| Error::from_raw_os_error(libc::EINVAL))?;

        let mut new_upper_real = None;
        parent_node.handle_upper_inode_locked(&mut |parent_upper_inode| -> Result<bool> {
            // We already create upper dir for parent_node above.
            let parent_real_inode =
                parent_upper_inode.ok_or_else(|| Error::from_raw_os_error(libc::EROFS))?;
            new_upper_real.replace(parent_real_inode.symlink(ctx, path, node.name.as_str())?);
            Ok(false)
        })?;

        if let Some(real_inode) = new_upper_real {
            // update upper_inode and first_inode()
            node.add_upper_inode(real_inode, true);
        }

        Ok(Arc::clone(&node))
    }

    // Copy regular file from lower layer to upper layer.
    // Caller must ensure node doesn't have upper layer.
    fn copy_regfile_up(&self, ctx: &Context, node: Arc<OverlayInode>) -> Result<Arc<OverlayInode>> {
        if node.in_upper_layer() {
            return Ok(node);
        }

        let parent_node = if let Some(ref n) = node.parent.lock().unwrap().upgrade() {
            Arc::clone(n)
        } else {
            return Err(Error::new(ErrorKind::Other, "no parent?"));
        };

        let st = node.stat64(ctx)?;
        let (lower_layer, _, lower_inode) = node.first_layer_inode();

        if !parent_node.in_upper_layer() {
            parent_node.create_upper_dir(ctx, None)?;
        }

        // create the file in upper layer using information from lower layer
        let args = CreateIn {
            flags: libc::O_WRONLY as u32,
            mode: st.st_mode,
            umask: 0,
            fuse_flags: 0,
        };

        let mut upper_handle = 0u64;
        let mut upper_real_inode = None;
        parent_node.handle_upper_inode_locked(&mut |parent_upper_inode| -> Result<bool> {
            // We already create upper dir for parent_node.
            let parent_real_inode = parent_upper_inode.ok_or_else(|| {
                error!("parent {} has no upper inode", parent_node.inode);
                Error::from_raw_os_error(libc::EINVAL)
            })?;
            let (inode, h) = parent_real_inode.create(ctx, node.name.as_str(), args)?;
            upper_handle = h.unwrap_or(0);
            upper_real_inode.replace(inode);
            Ok(false)
        })?;

        let (h, _, _) = lower_layer.open(ctx, lower_inode, libc::O_RDONLY as u32, 0)?;

        let lower_handle = h.unwrap_or(0);

        // need to use work directory and then rename file to
        // final destination for atomic reasons.. not deal with it for now,
        // use stupid copy at present.
        // FIXME: this need a lot of work here, ntimes, xattr, etc.

        // Copy from lower real inode to upper real inode.
        let mut file = tempfile::TempFile::new().unwrap().into_file();
        let mut offset: usize = 0;
        let size = 4 * 1024 * 1024;
        loop {
            let ret = lower_layer.read(
                ctx,
                lower_inode,
                lower_handle,
                &mut file,
                size,
                offset as u64,
                None,
                0,
            )?;
            if ret == 0 {
                break;
            }

            offset += ret;
        }
        // close handles
        lower_layer.release(ctx, lower_inode, 0, lower_handle, true, true, None)?;

        file.seek(SeekFrom::Start(0))?;
        offset = 0;

        while let Some(ref ri) = upper_real_inode {
            let ret = ri.layer.write(
                ctx,
                ri.inode,
                upper_handle,
                &mut file,
                size,
                offset as u64,
                None,
                false,
                0,
                0,
            )?;
            if ret == 0 {
                break;
            }

            offset += ret;
        }

        // Drop will remove file automatically.
        drop(file);

        if let Some(ri) = upper_real_inode {
            if let Err(e) = ri
                .layer
                .release(ctx, ri.inode, 0, upper_handle, true, true, None)
            {
                // Ignore ENOSYS.
                if e.raw_os_error() != Some(libc::ENOSYS) {
                    return Err(e);
                }
            }

            // update upper_inode and first_inode()
            node.add_upper_inode(ri, true);
        }

        Ok(Arc::clone(&node))
    }

    fn copy_node_up(&self, ctx: &Context, node: Arc<OverlayInode>) -> Result<Arc<OverlayInode>> {
        if node.in_upper_layer() {
            return Ok(node);
        }

        let st = node.stat64(ctx)?;
        // directory
        if utils::is_dir(st) {
            node.create_upper_dir(ctx, None)?;
            return Ok(Arc::clone(&node));
        }

        // For symlink.
        if st.st_mode & libc::S_IFMT == libc::S_IFLNK {
            return self.copy_symlink_up(ctx, Arc::clone(&node));
        }

        // For regular file.
        self.copy_regfile_up(ctx, Arc::clone(&node))
    }

    fn do_rm(&self, ctx: &Context, parent: u64, name: &CStr, dir: bool) -> Result<()> {
        if self.upper_layer.is_none() {
            return Err(Error::from_raw_os_error(libc::EROFS));
        }

        // Find parent Overlay Inode.
        let pnode = self.lookup_node(ctx, parent, "")?;
        if pnode.whiteout.load(Ordering::Relaxed) {
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        // Find the Overlay Inode for child with <name>.
        let sname = name.to_string_lossy().to_string();
        let node = self.lookup_node(ctx, parent, sname.as_str())?;
        if node.whiteout.load(Ordering::Relaxed) {
            // already deleted.
            return Err(Error::from_raw_os_error(libc::ENOENT));
        }

        if dir {
            self.load_directory(ctx, &node)?;
            let (count, whiteouts) = node.count_entries_and_whiteout(ctx)?;
            trace!("entries: {}, whiteouts: {}\n", count, whiteouts);
            if count > 0 {
                return Err(Error::from_raw_os_error(libc::ENOTEMPTY));
            }

            // Delete all whiteouts.
            if whiteouts > 0 && node.in_upper_layer() {
                self.empty_node_directory(ctx, Arc::clone(&node))?;
            }

            trace!("whiteouts deleted!\n");
        }

        let mut need_whiteout = true;
        let pnode = self.copy_node_up(ctx, Arc::clone(&pnode))?;

        if node.upper_layer_only() {
            need_whiteout = false;
        }

        let mut path_removed = None;
        if node.in_upper_layer() {
            pnode.handle_upper_inode_locked(&mut |parent_upper_inode| -> Result<bool> {
                let parent_real_inode = parent_upper_inode.ok_or_else(|| {
                    error!(
                        "BUG: parent {} has no upper inode after copy up",
                        pnode.inode
                    );
                    Error::from_raw_os_error(libc::EINVAL)
                })?;

                // Parent is opaque, it shadows everything in lower layers so no need to create extra whiteouts.
                if parent_real_inode.opaque {
                    need_whiteout = false;
                }
                if dir {
                    parent_real_inode
                        .layer
                        .rmdir(ctx, parent_real_inode.inode, name)?;
                } else {
                    parent_real_inode
                        .layer
                        .unlink(ctx, parent_real_inode.inode, name)?;
                }

                Ok(false)
            })?;

            path_removed.replace(node.path.clone());
        }

        trace!(
            "Remove inode {} from global hashmap and parent's children hashmap\n",
            node.inode
        );

        // lookups decrease by 1.
        node.lookups.fetch_sub(1, Ordering::Relaxed);

        // remove it from hashmap
        self.remove_inode(node.inode, path_removed);
        pnode.remove_child(node.name.as_str());

        if need_whiteout {
            trace!("do_rm: creating whiteout\n");
            // pnode is copied up, so it has upper layer.
            pnode.handle_upper_inode_locked(&mut |parent_upper_inode| -> Result<bool> {
                let parent_real_inode = parent_upper_inode.ok_or_else(|| {
                    error!(
                        "BUG: parent {} has no upper inode after copy up",
                        pnode.inode
                    );
                    Error::from_raw_os_error(libc::EINVAL)
                })?;

                let child_ri = parent_real_inode.create_whiteout(ctx, sname.as_str())?;
                let path = format!("{}/{}", pnode.path, sname);
                let ino = self.alloc_inode(&path)?;
                let ovi = Arc::new(OverlayInode::new_from_real_inode(
                    sname.as_str(),
                    ino,
                    path.clone(),
                    child_ri,
                ));

                self.insert_inode(ino, ovi.clone());
                pnode.insert_child(sname.as_str(), ovi.clone());
                Ok(false)
            })?;
        }

        Ok(())
    }

    fn do_fsync(
        &self,
        ctx: &Context,
        inode: Inode,
        datasync: bool,
        handle: Handle,
        syncdir: bool,
    ) -> Result<()> {
        // Use O_RDONLY flags which indicates no copy up.
        let data = self.get_data(ctx, Some(handle), inode, libc::O_RDONLY as u32)?;

        match data.real_handle {
            // FIXME: need to test if inode matches corresponding handle?
            None => Err(Error::from_raw_os_error(libc::ENOENT)),
            Some(ref rh) => {
                let real_handle = rh.handle.load(Ordering::Relaxed);
                // TODO: check if it's in upper layer? @weizhang555
                if syncdir {
                    rh.layer.fsyncdir(ctx, rh.inode, datasync, real_handle)
                } else {
                    rh.layer.fsync(ctx, rh.inode, datasync, real_handle)
                }
            }
        }
    }

    // Delete everything in the directory only on upper layer, ignore lower layers.
    fn empty_node_directory(&self, ctx: &Context, node: Arc<OverlayInode>) -> Result<()> {
        let st = node.stat64(ctx)?;
        if !utils::is_dir(st) {
            // This function can only be called on directories.
            return Err(Error::from_raw_os_error(libc::ENOTDIR));
        }

        let (layer, in_upper, inode) = node.first_layer_inode();
        if !in_upper {
            return Ok(());
        }

        // Copy node.childrens Hashmap to Vector, the Vector is also used as temp storage,
        // Without this, Rust won't allow us to remove them from node.childrens.
        let iter = node
            .childrens
            .lock()
            .unwrap()
            .iter()
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>();

        for child in iter {
            // We only care about upper layer, ignore lower layers.
            if child.in_upper_layer() {
                if child.whiteout.load(Ordering::Relaxed) {
                    layer.delete_whiteout(
                        ctx,
                        inode,
                        utils::to_cstring(child.name.as_str())?.as_c_str(),
                    )?
                } else {
                    let s = child.stat64(ctx)?;
                    let cname = utils::to_cstring(&child.name)?;
                    if utils::is_dir(s) {
                        let (count, whiteouts) = child.count_entries_and_whiteout(ctx)?;
                        if count + whiteouts > 0 {
                            self.empty_node_directory(ctx, Arc::clone(&child))?;
                        }

                        layer.rmdir(ctx, inode, cname.as_c_str())?
                    } else {
                        layer.unlink(ctx, inode, cname.as_c_str())?;
                    }
                }

                // delete the child
                self.remove_inode(child.inode, Some(child.path.clone()));
                node.remove_child(child.name.as_str());
            }
        }

        Ok(())
    }

    fn find_real_info_from_handle(
        &self,
        handle: Handle,
    ) -> Result<(Arc<BoxedLayer>, Inode, Handle)> {
        match self.handles.lock().unwrap().get(&handle) {
            Some(h) => match h.real_handle {
                Some(ref rhd) => Ok((
                    rhd.layer.clone(),
                    rhd.inode,
                    rhd.handle.load(Ordering::Relaxed),
                )),
                None => Err(Error::from_raw_os_error(libc::ENOENT)),
            },

            None => Err(Error::from_raw_os_error(libc::ENOENT)),
        }
    }

    fn find_real_inode(&self, inode: Inode) -> Result<(Arc<BoxedLayer>, Inode)> {
        if let Some(n) = self.get_active_inode(inode) {
            let (first_layer, _, first_inode) = n.first_layer_inode();
            return Ok((first_layer, first_inode));
        }

        Err(Error::from_raw_os_error(libc::ENOENT))
    }

    fn get_data(
        &self,
        ctx: &Context,
        handle: Option<Handle>,
        inode: Inode,
        flags: u32,
    ) -> Result<Arc<HandleData>> {
        let no_open = self.no_open.load(Ordering::Relaxed);
        if !no_open {
            if let Some(h) = handle {
                if let Some(v) = self.handles.lock().unwrap().get(&h) {
                    if v.node.inode == inode {
                        return Ok(Arc::clone(v));
                    }
                }
            }
        } else {
            let readonly: bool = flags
                & (libc::O_APPEND | libc::O_CREAT | libc::O_TRUNC | libc::O_RDWR | libc::O_WRONLY)
                    as u32
                == 0;

            // lookup node
            let node = self.lookup_node(ctx, inode, "")?;

            // whiteout node
            if node.whiteout.load(Ordering::Relaxed) {
                return Err(Error::from_raw_os_error(libc::ENOENT));
            }

            if !readonly {
                // Check if upper layer exists, return EROFS is not exists.
                self.upper_layer
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| Error::from_raw_os_error(libc::EROFS))?;
                // copy up to upper layer
                self.copy_node_up(ctx, Arc::clone(&node))?;
            }

            let (layer, in_upper_layer, inode) = node.first_layer_inode();
            let handle_data = HandleData {
                node: Arc::clone(&node),
                real_handle: Some(RealHandle {
                    layer,
                    in_upper_layer,
                    inode,
                    handle: AtomicU64::new(0),
                }),
            };
            return Ok(Arc::new(handle_data));
        }

        Err(Error::from_raw_os_error(libc::ENOENT))
    }

    
    // extend or init the inodes number to one overlay if the current number is done.
    pub fn extend_inode_alloc(&self,key:u64){
        let next_inode = key * INODE_ALLOC_BATCH;
        let limit_inode = next_inode + INODE_ALLOC_BATCH -1;
        self.inodes.write().unwrap().extend_inode_number(next_inode, limit_inode);
    }
}
#[cfg(not(feature = "async-io"))]
impl BackendFileSystem for OverlayFs {
    /// mount returns the backend file system root inode entry and
    /// the largest inode number it has.
    fn mount(&self) -> Result<(Entry, u64)> {
        let ctx = Context::default();
        let entry = self.do_lookup(&ctx, self.root_inode(), "")?;
        Ok((entry, VFS_MAX_INO))
    }

    /// Provides a reference to the Any trait. This is useful to let
    /// the caller have access to the underlying type behind the
    /// trait.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
#[cfg(test)]
mod tests {
    use std::{path::Path, thread};

    use crate::{passthrough::new_passthroughfs_layer, server::FuseServer};

    use super::*;
    use diff::FSdiff;
    use fuse_backend_rs::{api::server::Server, transport::{FuseChannel, FuseSession}};
    use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};
    #[derive(Debug, Default)]
    pub struct Args {
        name: String,
        mountpoint: String,
        lowerdir: Vec<String>,
        upperdir: String,
        workdir: String,
        #[allow(unused)]
        log_level: String,
    }
    
    #[test]
    fn test_overlayfs() {

        // Set up test environment
        let args = Args {
            name: "test_overlay".to_string(),
            mountpoint: "/home/luxian/megatest/true_temp".to_string(),
            lowerdir: vec!["/home/luxian/megatest/lower".to_string()],
            upperdir: "/home/luxian/megatest/upper".to_string(),
            workdir: "/home/luxian/megatest/workerdir".to_string(),
            log_level: "info".to_string(),
        };

        // Create lower layers
        let mut lower_layers = Vec::new();
        for lower in &args.lowerdir {
            let layer = new_passthroughfs_layer(lower).unwrap();
            lower_layers.push(Arc::new(layer));
        }
        // Create upper layer
        let upper_layer = Arc::new(new_passthroughfs_layer(&args.upperdir).unwrap());
        // Create overlayfs
        let  config = super::config::Config { 
            work: args.workdir.clone(), 
            mountpoint: args.mountpoint.clone(), 
            do_import: true, 
            ..Default::default() };
        
        let overlayfs = OverlayFs::new(Some(upper_layer), lower_layers, config,1).unwrap();
        // Import overlayfs
        overlayfs.import().unwrap();
        overlayfs.diff();
        // Create fuse session
        let mut se = FuseSession::new(Path::new(&args.mountpoint), &args.name, "", false).unwrap();
        se.mount().unwrap();
        // Create server
        let server = Arc::new(Server::new(Arc::new(overlayfs)));
        let ch: FuseChannel = se.new_channel().unwrap();


        let mut server = FuseServer {
            server,
            ch,
        };
        // Spawn server thread
        let handle = thread::spawn(move || {
            let _ = server.svc_loop();
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


