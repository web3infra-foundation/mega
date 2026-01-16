use std::{ffi::OsStr, num::NonZeroU32};

use rfuse3::{raw::prelude::*, Inode, Result};

use super::MegaFuse;
use crate::READONLY_INODE;
/// select the right fs by inodes .
/// 1. in inodes < READONLY_INODE , it from the readonly fuse.
/// 2. if inodes is a overlay fs root ,find it from the hashmap
/// 3. if inode from one  overlay, find it by batch number.
macro_rules! call_fuse_function {
    // &self  -> MegaFuse::Self
    // &func -> fuse_func , like `lookup` ,`init` , 'getattr'....
    // $req -> Request
    // $inode:expr -> Inode number . almost every func have this arg
    // $($args:expr),*  -> All other args
    ($self:ident,$func:ident, $req:expr , $inode:expr, $($args:expr),*) => {
        if let Some(ovl_inode) = $self.inodes_alloc.get_ovl_inode($inode/READONLY_INODE).await {
            if let Some(ovl_inode_root) = $self.overlayfs.lock().await.get(&ovl_inode){
                println!(" overlay child inode root");
                ovl_inode_root.$func($req,$inode,$($args),*).await
            }else{
                $self.dic.$func($req,$inode,$($args),*).await
            }
        }else if let Some(ovl_inode_root) = $self.overlayfs.lock().await.get(&$inode){
            println!(" overlay inode root");
            ovl_inode_root.$func($req,$inode,$($args),*).await
        }else if ($inode < READONLY_INODE){
            println!(" readonly inode root");
            $self.dic.$func($req,$inode,$($args),*).await
        }else{
            //TODO : don't panic, return error .
            panic!("can't find fs by inode");
        }
    };
}

impl Filesystem for MegaFuse {
    /// initialize filesystem. Called before any other filesystem method.
    async fn init(&self, req: Request) -> Result<ReplyInit> {
        let _ = self.dic.init(req).await;
        let map_lock = &self.overlayfs.lock().await;
        for (inode, ovl_fs) in map_lock.iter() {
            let inode_batch = self.inodes_alloc.alloc_inode(*inode).await;
            ovl_fs.extend_inode_alloc(inode_batch).await;
            let _ = ovl_fs.init(req).await;
        }
        Ok(ReplyInit {
            max_write: NonZeroU32::new(128 * 1024).unwrap(),
        })
    }

    /// clean up filesystem. Called on filesystem exit which is fuseblk, in normal fuse filesystem,
    /// kernel may call forget for root. There is some discuss for this
    /// <https://github.com/bazil/fuse/issues/82#issuecomment-88126886>,
    /// <https://sourceforge.net/p/fuse/mailman/message/31995737/>
    async fn destroy(&self, req: Request) {
        self.dic.destroy(req).await;
        let map_lock = &self.overlayfs.lock().await;
        for (_, ovl_fs) in map_lock.iter() {
            ovl_fs.destroy(req).await;
        }
    }

    /// look up a directory entry by name and get its attributes.
    async fn lookup(&self, req: Request, parent: Inode, name: &OsStr) -> Result<ReplyEntry> {
        call_fuse_function!(self, lookup, req, parent, name)
    }

    /// forget an inode. The nlookup parameter indicates the number of lookups previously
    /// performed on this inode. If the filesystem implements inode lifetimes, it is recommended
    /// that inodes acquire a single reference on each lookup, and lose nlookup references on each
    /// forget. The filesystem may ignore forget calls, if the inodes don't need to have a limited
    /// lifetime. On unmount it is not guaranteed, that all referenced inodes will receive a forget
    /// message. When filesystem is normal(not fuseblk) and unmounting, kernel may send forget
    /// request for root and this library will stop session after call forget. There is some
    /// discussion for this <https://github.com/bazil/fuse/issues/82#issuecomment-88126886>,
    /// <https://sourceforge.net/p/fuse/mailman/message/31995737/>
    async fn forget(&self, req: Request, inode: Inode, nlookup: u64) {
        call_fuse_function!(self, forget, req, inode, nlookup)
    }

    /// get file attributes. If `fh` is None, means `fh` is not set.
    async fn getattr(
        &self,
        req: Request,
        inode: Inode,
        fh: Option<u64>,
        flags: u32,
    ) -> Result<ReplyAttr> {
        call_fuse_function!(self, getattr, req, inode, fh, flags)
    }

    /// set file attributes. If `fh` is None, means `fh` is not set.
    async fn setattr(
        &self,
        req: Request,
        inode: Inode,
        fh: Option<u64>,
        set_attr: SetAttr,
    ) -> Result<ReplyAttr> {
        call_fuse_function!(self, setattr, req, inode, fh, set_attr)
    }

    /// read symbolic link.
    async fn readlink(&self, req: Request, inode: Inode) -> Result<ReplyData> {
        call_fuse_function!(self, readlink, req, inode,)
    }

    /// create a symbolic link.
    async fn symlink(
        &self,
        req: Request,
        parent: Inode,
        name: &OsStr,
        link: &OsStr,
    ) -> Result<ReplyEntry> {
        call_fuse_function!(self, symlink, req, parent, name, link)
    }

    /// create file node. Create a regular file, character device, block device, fifo or socket
    /// node. When creating file, most cases user only need to implement
    /// [`create`][Filesystem::create].
    async fn mknod(
        &self,
        req: Request,
        parent: Inode,
        name: &OsStr,
        mode: u32,
        rdev: u32,
    ) -> Result<ReplyEntry> {
        call_fuse_function!(self, mknod, req, parent, name, mode, rdev)
    }

    /// create a directory.
    async fn mkdir(
        &self,
        req: Request,
        parent: Inode,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> Result<ReplyEntry> {
        call_fuse_function!(self, mkdir, req, parent, name, mode, umask)
    }

    /// remove a file.
    async fn unlink(&self, req: Request, parent: Inode, name: &OsStr) -> Result<()> {
        call_fuse_function!(self, unlink, req, parent, name)
    }

    /// remove a directory.
    async fn rmdir(&self, req: Request, parent: Inode, name: &OsStr) -> Result<()> {
        call_fuse_function!(self, rmdir, req, parent, name)
    }

    /// rename a file or directory.
    async fn rename(
        &self,
        req: Request,
        parent: Inode,
        name: &OsStr,
        new_parent: Inode,
        new_name: &OsStr,
    ) -> Result<()> {
        call_fuse_function!(self, rename, req, parent, name, new_parent, new_name)
    }

    /// create a hard link.
    async fn link(
        &self,
        req: Request,
        inode: Inode,
        new_parent: Inode,
        new_name: &OsStr,
    ) -> Result<ReplyEntry> {
        call_fuse_function!(self, link, req, inode, new_parent, new_name)
    }

    /// open a file. Open flags (with the exception of `O_CREAT`, `O_EXCL` and `O_NOCTTY`) are
    /// available in flags. Filesystem may store an arbitrary file handle (pointer, index, etc) in
    /// fh, and use this in other all other file operations (read, write, flush, release, fsync).
    /// Filesystem may also implement stateless file I/O and not store anything in fh. There are
    /// also some flags (`direct_io`, `keep_cache`) which the filesystem may set, to change the way
    /// the file is opened. A filesystem need not implement this method if it
    /// sets [`MountOptions::no_open_support`][crate::MountOptions::no_open_support] and if the
    /// kernel supports `FUSE_NO_OPEN_SUPPORT`.
    ///
    /// # Notes:
    ///
    /// See `fuse_file_info` structure in
    /// [fuse_common.h](https://libfuse.github.io/doxygen/include_2fuse__common_8h_source.html) for
    /// more details.
    async fn open(&self, req: Request, inode: Inode, flags: u32) -> Result<ReplyOpen> {
        call_fuse_function!(self, open, req, inode, flags)
    }

    /// read data. Read should send exactly the number of bytes requested except on EOF or error,
    /// otherwise the rest of the data will be substituted with zeroes. An exception to this is
    /// when the file has been opened in `direct_io` mode, in which case the return value of the
    /// read system call will reflect the return value of this operation. `fh` will contain the
    /// value set by the open method, or will be undefined if the open method didn't set any value.
    async fn read(
        &self,
        req: Request,
        inode: Inode,
        fh: u64,
        offset: u64,
        size: u32,
    ) -> Result<ReplyData> {
        call_fuse_function!(self, read, req, inode, fh, offset, size)
    }

    /// write data. Write should return exactly the number of bytes requested except on error. An
    /// exception to this is when the file has been opened in `direct_io` mode, in which case the
    /// return value of the write system call will reflect the return value of this operation. `fh`
    /// will contain the value set by the open method, or will be undefined if the open method
    /// didn't set any value. When `write_flags` contains
    /// [`FUSE_WRITE_CACHE`](crate::raw::flags::FUSE_WRITE_CACHE), means the write operation is a
    /// delay write.
    #[allow(clippy::too_many_arguments)]
    async fn write(
        &self,
        req: Request,
        inode: Inode,
        fh: u64,
        offset: u64,
        data: &[u8],
        write_flags: u32,
        flags: u32,
    ) -> Result<ReplyWrite> {
        call_fuse_function!(
            self,
            write,
            req,
            inode,
            fh,
            offset,
            data,
            write_flags,
            flags
        )
    }

    /// get filesystem statistics.
    async fn statfs(&self, req: Request, inode: Inode) -> Result<ReplyStatFs> {
        call_fuse_function!(self, statfs, req, inode,)
    }

    /// release an open file. Release is called when there are no more references to an open file:
    /// all file descriptors are closed and all memory mappings are unmapped. For every open call
    /// there will be exactly one release call. The filesystem may reply with an error, but error
    /// values are not returned to `close()` or `munmap()` which triggered the release. `fh` will
    /// contain the value set by the open method, or will be undefined if the open method didn't
    /// set any value. `flags` will contain the same flags as for open. `flush` means flush the
    /// data or not when closing file.
    async fn release(
        &self,
        req: Request,
        inode: Inode,
        fh: u64,
        flags: u32,
        lock_owner: u64,
        flush: bool,
    ) -> Result<()> {
        call_fuse_function!(self, release, req, inode, fh, flags, lock_owner, flush)
    }

    /// synchronize file contents. If the `datasync` is true, then only the user data should be
    /// flushed, not the metadata.
    async fn fsync(&self, req: Request, inode: Inode, fh: u64, datasync: bool) -> Result<()> {
        call_fuse_function!(self, fsync, req, inode, fh, datasync)
    }

    /// set an extended attribute.
    async fn setxattr(
        &self,
        req: Request,
        inode: Inode,
        name: &OsStr,
        value: &[u8],
        flags: u32,
        position: u32,
    ) -> Result<()> {
        call_fuse_function!(self, setxattr, req, inode, name, value, flags, position)
    }

    /// Get an extended attribute. If `size` is too small, return `Err<ERANGE>`.
    /// Otherwise, use [`ReplyXAttr::Data`] to send the attribute data, or
    /// return an error.
    async fn getxattr(
        &self,
        req: Request,
        inode: Inode,
        name: &OsStr,
        size: u32,
    ) -> Result<ReplyXAttr> {
        call_fuse_function!(self, getxattr, req, inode, name, size)
    }

    /// List extended attribute names.
    ///
    /// If `size` is too small, return `Err<ERANGE>`.  Otherwise, use
    /// [`ReplyXAttr::Data`] to send the attribute list, or return an error.
    async fn listxattr(&self, req: Request, inode: Inode, size: u32) -> Result<ReplyXAttr> {
        call_fuse_function!(self, listxattr, req, inode, size)
    }

    /// remove an extended attribute.
    async fn removexattr(&self, req: Request, inode: Inode, name: &OsStr) -> Result<()> {
        call_fuse_function!(self, removexattr, req, inode, name)
    }

    /// flush method. This is called on each `close()` of the opened file. Since file descriptors
    /// can be duplicated (`dup`, `dup2`, `fork`), for one open call there may be many flush calls.
    /// Filesystems shouldn't assume that flush will always be called after some writes, or that if
    /// will be called at all. `fh` will contain the value set by the open method, or will be
    /// undefined if the open method didn't set any value.
    ///
    /// # Notes:
    ///
    /// the name of the method is misleading, since (unlike fsync) the filesystem is not forced to
    /// flush pending writes. One reason to flush data, is if the filesystem wants to return write
    /// errors. If the filesystem supports file locking operations ([`setlk`][Filesystem::setlk],
    /// [`getlk`][Filesystem::getlk]) it should remove all locks belonging to `lock_owner`.
    async fn flush(&self, req: Request, inode: Inode, fh: u64, lock_owner: u64) -> Result<()> {
        call_fuse_function!(self, flush, req, inode, fh, lock_owner)
    }

    /// open a directory. Filesystem may store an arbitrary file handle (pointer, index, etc) in
    /// `fh`, and use this in other all other directory stream operations
    /// ([`readdir`][Filesystem::readdir], [`releasedir`][Filesystem::releasedir],
    /// [`fsyncdir`][Filesystem::fsyncdir]). Filesystem may also implement stateless directory
    /// I/O and not store anything in `fh`.  A file system need not implement this method if it
    /// sets [`MountOptions::no_open_dir_support`][crate::MountOptions::no_open_dir_support] and
    /// if the kernel supports `FUSE_NO_OPENDIR_SUPPORT`.
    async fn opendir(&self, req: Request, inode: Inode, flags: u32) -> Result<ReplyOpen> {
        call_fuse_function!(self, opendir, req, inode, flags)
    }

    /// read directory. `offset` is used to track the offset of the directory entries. `fh` will
    /// contain the value set by the [`opendir`][Filesystem::opendir] method, or will be
    /// undefined if the [`opendir`][Filesystem::opendir] method didn't set any value.
    async fn readdir<'a>(
        &'a self,
        req: Request,
        parent: Inode,
        fh: u64,
        offset: i64,
    ) -> Result<ReplyDirectory<impl futures::Stream<Item = Result<DirectoryEntry>> + Send + 'a>>
    {
        use futures::StreamExt;

        if let Some(ovl_inode) = self
            .inodes_alloc
            .get_ovl_inode(parent / READONLY_INODE)
            .await
        {
            if let Some(ovl_inode_root) = self.overlayfs.lock().await.get(&ovl_inode).cloned() {
                let reply = ovl_inode_root.readdir(req, parent, fh, offset).await?;
                let entries: Vec<_> = reply.entries.collect().await;
                Ok(ReplyDirectory {
                    entries: futures::stream::iter(entries),
                })
            } else {
                let reply = self.dic.readdir(req, parent, fh, offset).await?;
                let entries: Vec<_> = reply.entries.collect().await;
                Ok(ReplyDirectory {
                    entries: futures::stream::iter(entries),
                })
            }
        } else if let Some(ovl_inode_root) = self.overlayfs.lock().await.get(&parent).cloned() {
            let reply = ovl_inode_root.readdir(req, parent, fh, offset).await?;
            let entries: Vec<_> = reply.entries.collect().await;
            Ok(ReplyDirectory {
                entries: futures::stream::iter(entries),
            })
        } else if parent < READONLY_INODE {
            let reply = self.dic.readdir(req, parent, fh, offset).await?;
            let entries: Vec<_> = reply.entries.collect().await;
            Ok(ReplyDirectory {
                entries: futures::stream::iter(entries),
            })
        } else {
            panic!("can't find fs by inode");
        }
    }

    /// release an open directory. For every [`opendir`][Filesystem::opendir] call there will
    /// be exactly one `releasedir` call. `fh` will contain the value set by the
    /// [`opendir`][Filesystem::opendir] method, or will be undefined if the
    /// [`opendir`][Filesystem::opendir] method didn't set any value.
    async fn releasedir(&self, req: Request, inode: Inode, fh: u64, flags: u32) -> Result<()> {
        call_fuse_function!(self, releasedir, req, inode, fh, flags)
    }

    /// synchronize directory contents. If the `datasync` is true, then only the directory contents
    /// should be flushed, not the metadata. `fh` will contain the value set by the
    /// [`opendir`][Filesystem::opendir] method, or will be undefined if the
    /// [`opendir`][Filesystem::opendir] method didn't set any value.
    async fn fsyncdir(&self, req: Request, inode: Inode, fh: u64, datasync: bool) -> Result<()> {
        call_fuse_function!(self, fsyncdir, req, inode, fh, datasync)
    }
    /// check file access permissions. This will be called for the `access()` system call. If the
    /// `default_permissions` mount option is given, this method is not be called. This method is
    /// not called under Linux kernel versions 2.4.x.
    async fn access(&self, req: Request, inode: Inode, mask: u32) -> Result<()> {
        call_fuse_function!(self, access, req, inode, mask)
    }

    /// create and open a file. If the file does not exist, first create it with the specified
    /// mode, and then open it. Open flags (with the exception of `O_NOCTTY`) are available in
    /// flags. Filesystem may store an arbitrary file handle (pointer, index, etc) in `fh`, and use
    /// this in other all other file operations ([`read`][Filesystem::read],
    /// [`write`][Filesystem::write], [`flush`][Filesystem::flush],
    /// [`release`][Filesystem::release], [`fsync`][Filesystem::fsync]). There are also some flags
    /// (`direct_io`, `keep_cache`) which the filesystem may set, to change the way the file is
    /// opened. If this method is not implemented or under Linux kernel versions earlier than
    /// 2.6.15, the [`mknod`][Filesystem::mknod] and [`open`][Filesystem::open] methods will be
    /// called instead.
    ///
    /// # Notes:
    ///
    /// See `fuse_file_info` structure in
    /// [fuse_common.h](https://libfuse.github.io/doxygen/include_2fuse__common_8h_source.html) for
    /// more details.
    async fn create(
        &self,
        req: Request,
        parent: Inode,
        name: &OsStr,
        mode: u32,
        flags: u32,
    ) -> Result<ReplyCreated> {
        call_fuse_function!(self, create, req, parent, name, mode, flags)
    }

    /// handle interrupt. When a operation is interrupted, an interrupt request will send to fuse
    /// server with the unique id of the operation.
    async fn interrupt(&self, req: Request, unique: u64) -> Result<()> {
        call_fuse_function!(self, interrupt, req, unique,)
    }

    /// map block index within file to block index within device.
    ///
    /// # Notes:
    ///
    /// This may not works because currently this crate doesn't support fuseblk mode yet.
    async fn bmap(
        &self,
        req: Request,
        inode: Inode,
        blocksize: u32,
        idx: u64,
    ) -> Result<ReplyBmap> {
        call_fuse_function!(self, bmap, req, inode, blocksize, idx)
    }

    /// forget more than one inode. This is a batch version [`forget`][Filesystem::forget]
    async fn batch_forget(&self, req: Request, inodes: &[(u64, u64)]) {
        for (inode, vlookup) in inodes.iter() {
            self.forget(req, *inode, *vlookup).await
        }
    }

    /// allocate space for an open file. This function ensures that required space is allocated for
    /// specified file.
    ///
    /// # Notes:
    ///
    /// more information about `fallocate`, please see **`man 2 fallocate`**
    async fn fallocate(
        &self,
        req: Request,
        inode: Inode,
        fh: u64,
        offset: u64,
        length: u64,
        mode: u32,
    ) -> Result<()> {
        call_fuse_function!(self, fallocate, req, inode, fh, offset, length, mode)
    }

    /// read directory entries, but with their attribute, like [`readdir`][Filesystem::readdir]
    /// + [`lookup`][Filesystem::lookup] at the same time.
    async fn readdirplus<'a>(
        &'a self,
        req: Request,
        parent: Inode,
        fh: u64,
        offset: u64,
        lock_owner: u64,
    ) -> Result<
        ReplyDirectoryPlus<impl futures::Stream<Item = Result<DirectoryEntryPlus>> + Send + 'a>,
    > {
        use futures::StreamExt;

        if let Some(ovl_inode) = self
            .inodes_alloc
            .get_ovl_inode(parent / READONLY_INODE)
            .await
        {
            if let Some(ovl_inode_root) = self.overlayfs.lock().await.get(&ovl_inode).cloned() {
                let reply = ovl_inode_root
                    .readdirplus(req, parent, fh, offset, lock_owner)
                    .await?;
                let entries: Vec<_> = reply.entries.collect().await;
                Ok(ReplyDirectoryPlus {
                    entries: futures::stream::iter(entries),
                })
            } else {
                let reply = self
                    .dic
                    .readdirplus(req, parent, fh, offset, lock_owner)
                    .await?;
                let entries: Vec<_> = reply.entries.collect().await;
                Ok(ReplyDirectoryPlus {
                    entries: futures::stream::iter(entries),
                })
            }
        } else if let Some(ovl_inode_root) = self.overlayfs.lock().await.get(&parent).cloned() {
            let reply = ovl_inode_root
                .readdirplus(req, parent, fh, offset, lock_owner)
                .await?;
            let entries: Vec<_> = reply.entries.collect().await;
            Ok(ReplyDirectoryPlus {
                entries: futures::stream::iter(entries),
            })
        } else if parent < READONLY_INODE {
            let reply = self
                .dic
                .readdirplus(req, parent, fh, offset, lock_owner)
                .await?;
            let entries: Vec<_> = reply.entries.collect().await;
            Ok(ReplyDirectoryPlus {
                entries: futures::stream::iter(entries),
            })
        } else {
            panic!("can't find fs by inode");
        }
    }

    /// rename a file or directory with flags.
    async fn rename2(
        &self,
        req: Request,
        parent: Inode,
        name: &OsStr,
        new_parent: Inode,
        new_name: &OsStr,
        flags: u32,
    ) -> Result<()> {
        call_fuse_function!(self, rename2, req, parent, name, new_parent, new_name, flags)
    }

    /// find next data or hole after the specified offset.
    async fn lseek(
        &self,
        req: Request,
        inode: Inode,
        fh: u64,
        offset: u64,
        whence: u32,
    ) -> Result<ReplyLSeek> {
        call_fuse_function!(self, lseek, req, inode, fh, offset, whence)
    }
}
