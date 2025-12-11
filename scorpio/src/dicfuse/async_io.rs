use std::ffi::OsStr;
use std::num::NonZeroU32;

use bytes::Bytes;
use rfuse3::notify::Notify;
use rfuse3::raw::prelude::*;
use rfuse3::raw::reply::DirectoryEntry;
use rfuse3::{Errno, Inode, Result};

use super::Dicfuse;
use crate::dicfuse::store::load_dir;
use futures::stream::iter;
impl Filesystem for Dicfuse {
    /// initialize filesystem. Called before any other filesystem method.
    async fn init(&self, _req: Request) -> Result<ReplyInit> {
        let s = self.store.clone();
        super::store::import_arc(s).await; // This task can be spawned
        Ok(ReplyInit {
            max_write: NonZeroU32::new(128 * 1024).unwrap(),
        })
    }

    async fn getattr(
        &self,
        _req: Request,
        inode: Inode,
        _fh: Option<u64>,
        _flags: u32,
    ) -> Result<ReplyAttr> {
        let item = self.store.get_inode(inode).await?;
        let re = self.get_stat(item).await;
        Ok(ReplyAttr {
            attr: re.attr,
            ttl: re.ttl,
        })
    }

    /// clean up filesystem. Called on filesystem exit which is fuseblk, in normal fuse filesystem,
    /// kernel may call forget for root. There is some discuss for this
    /// <https://github.com/bazil/fuse/issues/82#issuecomment-88126886>,
    /// <https://sourceforge.net/p/fuse/mailman/message/31995737/>
    async fn destroy(&self, _req: Request) {}

    /// look up a directory entry by name and get its attributes.
    async fn lookup(&self, _req: Request, parent: Inode, name: &OsStr) -> Result<ReplyEntry> {
        let store = self.store.clone();
        let mut ppath = store
            .find_path(parent)
            .await
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENODATA))?;

        ppath.push(name.to_string_lossy().into_owned());
        let child = store.get_by_path(&ppath.to_string()).await?;
        let re = self.get_stat(child).await;
        Ok(re)
    }
    async fn mknod(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _rdev: u32,
    ) -> Result<ReplyEntry> {
        Err(libc::ENOSYS.into())
    }

    async fn statfs(&self, _req: Request, _inode: Inode) -> Result<ReplyStatFs> {
        // Return simple filesystem statistics
        Ok(ReplyStatFs {
            blocks: 1024 * 1024, // Total blocks
            bfree: 1024 * 512,   // Free blocks
            bavail: 1024 * 512,  // Free blocks for unprivileged users
            files: 100_000,      // Total file nodes
            ffree: 99_000,       // Free file nodes
            bsize: 4096,         // Block size
            namelen: 255,        // Maximum filename length
            frsize: 4096,        // Fragment size
        })
    }

    async fn release(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> Result<()> {
        Ok(())
    }

    async fn fsync(&self, _req: Request, _inode: Inode, _fh: u64, _datasync: bool) -> Result<()> {
        Ok(())
    }

    async fn setxattr(
        &self,
        _req: Request,
        _inode: Inode,
        _name: &OsStr,
        _value: &[u8],
        _flags: u32,
        _position: u32,
    ) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn getxattr(
        &self,
        _req: Request,
        _inode: Inode,
        _name: &OsStr,
        _size: u32,
    ) -> Result<ReplyXAttr> {
        // Dicfuse is a read-only filesystem and does not support extended attributes
        // Return ENODATA to indicate the requested attribute does not exist
        // This allows is_opaque function to correctly determine directories are not opaque
        Err(std::io::Error::from_raw_os_error(libc::ENODATA).into())
    }

    async fn listxattr(&self, _req: Request, _inode: Inode, _size: u32) -> Result<ReplyXAttr> {
        Err(libc::ENOSYS.into())
    }

    async fn removexattr(&self, _req: Request, _inode: Inode, _name: &OsStr) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn flush(&self, _req: Request, _inode: Inode, _fh: u64, _lock_owner: u64) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn unlink(&self, _req: Request, _parent: Inode, _name: &OsStr) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn rmdir(&self, _req: Request, _parent: Inode, _name: &OsStr) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn rename(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _new_parent: Inode,
        _new_name: &OsStr,
    ) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn mkdir(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
    ) -> Result<ReplyEntry> {
        Err(libc::ENOSYS.into())
    }

    async fn link(
        &self,
        _req: Request,
        _inode: Inode,
        _new_parent: Inode,
        _new_name: &OsStr,
    ) -> Result<ReplyEntry> {
        Err(libc::ENOSYS.into())
    }
    /// open a directory. Filesystem may store an arbitrary file handle (pointer, index, etc) in
    /// `fh`, and use this in other all other directory stream operations
    /// ([`readdir`][Filesystem::readdir], [`releasedir`][Filesystem::releasedir],
    /// [`fsyncdir`][Filesystem::fsyncdir]). Filesystem may also implement stateless directory
    /// I/O and not store anything in `fh`.  A file system need not implement this method if it
    /// sets [`MountOptions::no_open_dir_support`][crate::MountOptions::no_open_dir_support] and
    /// if the kernel supports `FUSE_NO_OPENDIR_SUPPORT`.
    async fn opendir(&self, _req: Request, _inode: Inode, _flags: u32) -> Result<ReplyOpen> {
        Ok(ReplyOpen { fh: 0, flags: 0 })
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
    async fn open(&self, _req: Request, inode: Inode, _flags: u32) -> Result<ReplyOpen> {
        println!("open a new readonly one inode {inode}");
        // let trees = fetch_tree();
        Ok(ReplyOpen { fh: 0, flags: 0 })
    }
    /// read data. Read should send exactly the number of bytes requested except on EOF or error,
    /// otherwise the rest of the data will be substituted with zeroes. An exception to this is
    /// when the file has been opened in `direct_io` mode, in which case the return value of the
    /// read system call will reflect the return value of this operation. `fh` will contain the
    /// value set by the open method, or will be undefined if the open method didn't set any value.
    async fn read(
        &self,
        _req: Request,
        inode: Inode,
        _fh: u64,
        offset: u64,
        size: u32,
    ) -> Result<ReplyData> {
        if !self.readable {
            return Ok(ReplyData {
                data: Bytes::from("".as_bytes()),
            });
        }
        let datas = self
            .store
            .get_file_content(inode)
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
        let _offset = offset as usize;
        let end = (_offset + size as usize).min(datas.len());
        let slice = &datas[_offset..end];
        //println!("read result :{:?}",slice);
        Ok(ReplyData {
            data: Bytes::copy_from_slice(slice),
        })
    }
    async fn access(&self, _req: Request, inode: Inode, _mask: u32) -> Result<()> {
        // Verify inode exists
        self.store.get_inode(inode).await?;

        // Try to get path and load directory contents
        if let Some(path) = self.store.find_path(inode).await {
            let load_parent = "/".to_string() + &path.to_string();
            let max_depth = self.store.max_depth() + load_parent.matches('/').count();
            match load_dir(self.store.clone(), load_parent, max_depth).await {
                Ok(true) => {
                    self.store.update_ancestors_hash(inode).await;
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!("load_dir failed for inode {}: {}", inode, e);
                }
            }
        }
        // Return success as long as inode exists, even if find_path returns None
        Ok(())
    }

    async fn write(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _offset: u64,
        data: &[u8],
        _write_flags: u32,
        _flags: u32,
    ) -> Result<ReplyWrite> {
        Ok(ReplyWrite {
            written: data.len() as u32,
        })
    }

    async fn releasedir(&self, _req: Request, _inode: Inode, _fh: u64, _flags: u32) -> Result<()> {
        Ok(())
    }

    async fn fsyncdir(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _datasync: bool,
    ) -> Result<()> {
        Ok(())
    }

    async fn readdir<'a>(
        &'a self,
        _req: Request,
        parent: Inode,
        fh: u64,
        offset: i64,
    ) -> Result<ReplyDirectory<impl futures::Stream<Item = Result<DirectoryEntry>> + Send + 'a>>
    {
        let all_items = self.store.do_readdir(parent, fh, 0).await?;
        let mut d: Vec<std::result::Result<DirectoryEntry, Errno>> = Vec::new();

        // do_readdir returns: [0]=current dir (.), [1]=parent dir (..), [2..]=children
        let parent_parent_inode = if all_items.len() >= 2 {
            all_items[1].get_inode() // Parent directory's inode for ..
        } else {
            parent // For root directory, .. points to itself
        };

        let items = if all_items.len() >= 2 {
            all_items[2..].to_vec() // Skip first two, get actual children
        } else {
            all_items[..].to_vec()
        };

        let parent_item = self.store.get_inode(parent).await?;
        self.load_files(parent_item, &items).await;

        // offset 0: ".", offset 1: "..", offset 2+: actual entries
        if offset < 1 {
            d.push(Ok(DirectoryEntry {
                inode: parent, // . points to current directory
                kind: rfuse3::FileType::Directory,
                name: ".".into(),
                offset: 1,
            }));
        }

        if offset < 2 {
            d.push(Ok(DirectoryEntry {
                inode: parent_parent_inode, // .. points to parent directory
                kind: rfuse3::FileType::Directory,
                name: "..".into(),
                offset: 2,
            }));
        }

        for (index, item) in items.iter().enumerate() {
            let entry_offset = (index + 2) as i64;
            if entry_offset > offset {
                d.push(Ok(DirectoryEntry {
                    inode: item.get_inode(),
                    kind: item.get_filetype().await,
                    name: item.get_name().into(),
                    offset: entry_offset + 1,
                }));
            }
        }
        Ok(ReplyDirectory {
            entries: iter(d.into_iter()),
        })
    }

    async fn readdirplus<'a>(
        &'a self,
        _req: Request,
        parent: Inode,
        fh: u64,
        offset: u64,
        _lock_owner: u64,
    ) -> Result<
        ReplyDirectoryPlus<impl futures::Stream<Item = Result<DirectoryEntryPlus>> + Send + 'a>,
    > {
        let all_items = self.store.do_readdir(parent, fh, 0).await?;
        let mut d: Vec<std::result::Result<DirectoryEntryPlus, Errno>> = Vec::new();

        // do_readdir returns: [0]=current dir (.), [1]=parent dir (..), [2..]=children
        let (parent_parent_item, parent_parent_inode) = if all_items.len() >= 2 {
            (all_items[1].clone(), all_items[1].get_inode())
        } else {
            let parent_item = self.store.get_inode(parent).await?;
            (parent_item.clone(), parent)
        };

        let items = if all_items.len() >= 2 {
            all_items[2..].to_vec() // Skip first two
        } else {
            all_items[..].to_vec()
        };

        let parent_item = self.store.get_inode(parent).await?;
        let parent_attr = self.get_stat(parent_item).await;
        let parent_parent_attr = self.get_stat(parent_parent_item).await;

        // offset 0: ".", offset 1: "..", offset 2+: actual entries
        if offset < 1 {
            d.push(Ok(DirectoryEntryPlus {
                inode: parent, // . points to current directory
                kind: rfuse3::FileType::Directory,
                name: ".".into(),
                offset: 1,
                generation: 0,
                attr: parent_attr.attr,
                entry_ttl: parent_attr.ttl,
                attr_ttl: parent_attr.ttl,
            }));
        }

        if offset < 2 {
            d.push(Ok(DirectoryEntryPlus {
                inode: parent_parent_inode, // .. points to parent directory
                kind: rfuse3::FileType::Directory,
                name: "..".into(),
                offset: 2,
                generation: 0,
                attr: parent_parent_attr.attr,
                entry_ttl: parent_parent_attr.ttl,
                attr_ttl: parent_parent_attr.ttl,
            }));
        }

        for (index, item) in items.iter().enumerate() {
            let entry_offset = (index + 2) as i64;
            if entry_offset > offset as i64 {
                let attr = self.get_stat(item.clone()).await;
                d.push(Ok(DirectoryEntryPlus {
                    inode: item.get_inode(),
                    kind: item.get_filetype().await,
                    name: item.get_name().into(),
                    offset: entry_offset + 1,
                    generation: 0,
                    attr: attr.attr,
                    entry_ttl: attr.ttl,
                    attr_ttl: attr.ttl,
                }));
            }
        }
        Ok(ReplyDirectoryPlus {
            entries: iter(d.into_iter()),
        })
    }

    async fn create(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _flags: u32,
    ) -> Result<ReplyCreated> {
        Err(libc::ENOSYS.into())
    }

    async fn interrupt(&self, _req: Request, _unique: u64) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn bmap(
        &self,
        _req: Request,
        _inode: Inode,
        _blocksize: u32,
        _idx: u64,
    ) -> Result<ReplyBmap> {
        Err(libc::ENOSYS.into())
    }

    async fn poll(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _kh: Option<u64>,
        _flags: u32,
        _events: u32,
        _notify: &Notify,
    ) -> Result<ReplyPoll> {
        Err(libc::ENOSYS.into())
    }

    async fn notify_reply(
        &self,
        _req: Request,
        _inode: Inode,
        _offset: u64,
        _data: Bytes,
    ) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn forget(&self, _req: Request, _inode: Inode, _nlookup: u64) {}

    async fn batch_forget(&self, _req: Request, _inodes: &[(Inode, u64)]) {
        for (_inode, _nlookup) in _inodes.iter() {
            self.forget(_req, *_inode, *_nlookup).await;
        }
    }

    async fn fallocate(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _offset: u64,
        _length: u64,
        _mode: u32,
    ) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn rename2(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _new_parent: Inode,
        _new_name: &OsStr,
        _flags: u32,
    ) -> Result<()> {
        Err(libc::ENOSYS.into())
    }

    async fn lseek(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _offset: u64,
        _whence: u32,
    ) -> Result<ReplyLSeek> {
        Err(libc::ENOSYS.into())
    }

    async fn copy_file_range(
        &self,
        _req: Request,
        _inode: Inode,
        _fh_in: u64,
        _off_in: u64,
        _inode_out: Inode,
        _fh_out: u64,
        _off_out: u64,
        _length: u64,
        _flags: u64,
    ) -> Result<ReplyCopyFileRange> {
        Err(libc::ENOSYS.into())
    }
}
