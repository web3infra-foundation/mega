use std::ffi::OsStr;
use std::num::NonZeroU32;

use bytes::Bytes;
use fuse3::raw::prelude::*;
use fuse3::raw::reply::DirectoryEntry;
use fuse3::{Errno, Inode, Result};

use futures::stream::{iter, Iter};
use std::vec::IntoIter;

use super::Dicfuse;

impl Filesystem for Dicfuse {
    /// dir entry stream given by [`readdir`][Filesystem::readdir].
    type DirEntryStream<'a>
        = Iter<IntoIter<Result<DirectoryEntry>>>
    where
        Self: 'a;
    /// dir entry stream given by [`readdir`][Filesystem::readdir].
    type DirEntryPlusStream<'a>
        = Iter<IntoIter<Result<DirectoryEntryPlus>>>
    where
        Self: 'a;

    /// look up a directory entry by name and get its attributes.
    async fn lookup(&self, _req: Request, parent: Inode, name: &OsStr) -> Result<ReplyEntry> {
        let store = self.store.clone();
        let mut ppath = store
            .find_path(parent)
            .await
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENODATA))?;

        ppath.push(name.to_string_lossy().into_owned());
        let chil = store.get_by_path(&ppath.to_string()).await?;
        if self.open_buff.read().await.get(&chil.get_inode()).is_none() {
            let _ = self.load_one_file(parent, name).await;
        }
        let re = self.get_stat(chil).await;
        Ok(re)
    }
    /// initialize filesystem. Called before any other filesystem method.
    async fn init(&self, _req: Request) -> Result<ReplyInit> {
        let s = self.store.clone();
        super::store::import_arc(s).await; // This task can be spawned
        Ok(ReplyInit {
            max_write: NonZeroU32::new(128 * 1024).unwrap(),
        })
    }

    /// clean up filesystem. Called on filesystem exit which is fuseblk, in normal fuse filesystem,
    /// kernel may call forget for root. There is some discuss for this
    /// <https://github.com/bazil/fuse/issues/82#issuecomment-88126886>,
    /// <https://sourceforge.net/p/fuse/mailman/message/31995737/>
    async fn destroy(&self, _req: Request) {}

    /// get file attributes. If `fh` is None, means `fh` is not set.
    async fn getattr(
        &self,
        _req: Request,
        inode: Inode,
        _fh: Option<u64>,
        _flags: u32,
    ) -> Result<ReplyAttr> {
        let store = self.store.clone();
        let _i = store
            .find_path(inode)
            .await
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENODATA))?;
        let item = store.get_inode(inode).await?;
        let e = self.get_stat(item).await;
        Ok(ReplyAttr {
            ttl: e.ttl,
            attr: e.attr,
        })
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
        println!("open a new readonly one inode {}", inode);
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
        let read_lock = self.open_buff.read().await;
        let datas = read_lock
            .get(&inode)
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
        self.store.get_inode(inode).await?;
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
    async fn readdir(
        &self,
        _req: Request,
        parent: Inode,
        fh: u64,
        offset: i64,
    ) -> Result<ReplyDirectory<Self::DirEntryStream<'_>>> {
        let items = self.store.do_readdir(parent, fh, offset as u64).await?;
        let mut d: Vec<std::result::Result<DirectoryEntry, Errno>> = Vec::new();

        let parent_item = self.store.get_inode(parent).await?;
        self.load_files(parent_item, &items).await;

        for (index, item) in items.into_iter().enumerate() {
            d.push(Ok(DirectoryEntry {
                inode: item.get_inode(),
                kind: item.get_filetype().await,
                name: item.get_name().into(),
                offset: (index + 1) as i64,
            }));
        }
        Ok(ReplyDirectory {
            entries: iter(d.into_iter()),
        })
    }
    async fn readdirplus(
        &self,
        _req: Request,
        parent: Inode,
        fh: u64,
        offset: u64,
        _lock_owner: u64,
    ) -> Result<ReplyDirectoryPlus<Self::DirEntryPlusStream<'_>>> {
        let items = self.store.do_readdir(parent, fh, offset).await?;
        let mut d: Vec<std::result::Result<DirectoryEntryPlus, Errno>> = Vec::new();

        let parent_item = self.store.get_inode(parent).await?;
        self.load_files(parent_item, &items).await;
        for (index, item) in items.into_iter().enumerate() {
            if index as u64 >= offset {
                let attr = self.get_stat(item.clone()).await;
                let e_name = if index == 0 {
                    String::from(".")
                } else if index == 1 {
                    String::from("..")
                } else {
                    item.get_name()
                };
                d.push(Ok(DirectoryEntryPlus {
                    inode: item.get_inode(),
                    kind: item.get_filetype().await,
                    name: e_name.into(),
                    offset: (index + 1) as i64,
                    generation: 0,
                    attr: attr.attr,
                    entry_ttl: attr.ttl,
                    attr_ttl: attr.ttl,
                }));
            }
        }
        println!("{:?}", d);
        Ok(ReplyDirectoryPlus {
            entries: iter(d.into_iter()),
        })
    }
}
