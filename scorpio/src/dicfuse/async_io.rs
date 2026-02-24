use std::{ffi::OsStr, num::NonZeroU32, time::Duration};

use bytes::Bytes;
use futures::stream::iter;
use rfuse3::{
    notify::Notify,
    raw::{prelude::*, reply::DirectoryEntry},
    Errno, Inode, Result,
};

use super::Dicfuse;
use crate::dicfuse::{
    abi::{default_dic_entry, default_file_entry},
    store::EMPTY_BLOB_OID,
};
impl Filesystem for Dicfuse {
    /// initialize filesystem. Called before any other filesystem method.
    async fn init(&self, _req: Request) -> Result<ReplyInit> {
        let s = self.store.clone();

        // Spawn import_arc as a background task to avoid blocking FUSE mount.
        // Guarded so we don't start multiple concurrent imports for the same store
        // (DicfuseManager may already have started one).
        if s.try_start_import() {
            tokio::spawn(async move {
                super::store::import_arc(s).await;
            });
        }

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

    async fn setattr(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: Option<u64>,
        _set_attr: SetAttr,
    ) -> Result<ReplyAttr> {
        // Read-only filesystem: deny metadata mutation (chmod/chown/utimens/truncate, etc).
        Err(libc::EROFS.into())
    }

    /// clean up filesystem. Called on filesystem exit which is fuseblk, in normal fuse filesystem,
    /// kernel may call forget for root. There is some discuss for this
    /// <https://github.com/bazil/fuse/issues/82#issuecomment-88126886>,
    /// <https://sourceforge.net/p/fuse/mailman/message/31995737/>
    async fn destroy(&self, _req: Request) {}

    /// look up a directory entry by name and get its attributes.
    async fn lookup(&self, _req: Request, parent: Inode, name: &OsStr) -> Result<ReplyEntry> {
        // Keep lookup mostly non-blocking: wait a short budget for directory refresh,
        // then continue with best-effort cache lookup and only retry once on miss.
        const LOOKUP_REFRESH_WAIT_BUDGET_MS: u64 = 20;
        const LOOKUP_MISS_RETRY_WAIT_BUDGET_MS: u64 = 200;

        let store = self.store.clone();

        let mut refresh_handle = None;
        let mut refresh_timed_out = false;

        let refresh_needed = match store.dir_refresh_needed(parent) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "dicfuse: check dir refresh state for parent inode {} failed: {}",
                    parent,
                    e
                );
                true
            }
        };

        if refresh_needed {
            let store_for_refresh = store.clone();
            let mut handle =
                tokio::spawn(async move { store_for_refresh.ensure_dir_loaded(parent).await });

            match tokio::time::timeout(
                Duration::from_millis(LOOKUP_REFRESH_WAIT_BUDGET_MS),
                &mut handle,
            )
            .await
            {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(e))) => {
                    tracing::warn!(
                        "dicfuse: refresh parent inode {} before lookup failed: {}",
                        parent,
                        e
                    );
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "dicfuse: refresh task join failed for parent inode {}: {}",
                        parent,
                        e
                    );
                }
                Err(_) => {
                    // The spawned task continues running in the background even if this handle
                    // is later dropped (cache-hit path). This is intentional: the refresh will
                    // populate the cache for subsequent lookups.
                    refresh_timed_out = true;
                    refresh_handle = Some(handle);
                }
            }
        }

        let mut ppath = store
            .find_path(parent)
            .await
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENODATA))?;
        ppath.push(name.to_string_lossy().into_owned());
        let child_path = ppath.to_string();

        let mut child = match store.get_by_path(&child_path).await {
            Ok(v) => Some(v),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };

        if child.is_none() && refresh_timed_out {
            if let Some(handle) = refresh_handle.as_mut() {
                match tokio::time::timeout(
                    Duration::from_millis(LOOKUP_MISS_RETRY_WAIT_BUDGET_MS),
                    handle,
                )
                .await
                {
                    Ok(Ok(Ok(()))) => {}
                    Ok(Ok(Err(e))) => {
                        tracing::warn!(
                            "dicfuse: refresh parent inode {} after miss failed: {}",
                            parent,
                            e
                        );
                    }
                    Ok(Err(e)) => {
                        tracing::warn!(
                            "dicfuse: refresh task join after miss failed for parent inode {}: {}",
                            parent,
                            e
                        );
                    }
                    Err(_) => {}
                }
            }

            child = match store.get_by_path(&child_path).await {
                Ok(v) => Some(v),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
        }

        let child = match child {
            Some(v) => v,
            None => {
                // TODO(perf): add short-lived negative lookup cache for ENOENT
                // to avoid repeated misses for Buck2 probe paths.
                return Err(std::io::Error::from_raw_os_error(libc::ENOENT).into());
            }
        };

        let re = self.get_stat(child).await;
        Ok(re)
    }

    async fn symlink(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _link: &OsStr,
    ) -> Result<ReplyEntry> {
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
    }

    async fn mknod(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _rdev: u32,
    ) -> Result<ReplyEntry> {
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
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
        // Read-only filesystem: deny metadata mutation.
        Err(libc::EROFS.into())
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

    async fn listxattr(&self, _req: Request, _inode: Inode, size: u32) -> Result<ReplyXAttr> {
        // Dicfuse does not expose extended attributes. Return an empty list.
        if size == 0 {
            Ok(ReplyXAttr::Size(0))
        } else {
            Ok(ReplyXAttr::Data(Bytes::new()))
        }
    }

    async fn removexattr(&self, _req: Request, _inode: Inode, _name: &OsStr) -> Result<()> {
        // Read-only filesystem: deny metadata mutation.
        Err(libc::EROFS.into())
    }

    async fn flush(&self, _req: Request, _inode: Inode, _fh: u64, _lock_owner: u64) -> Result<()> {
        // No-op for read-only filesystem.
        Ok(())
    }

    async fn unlink(&self, _req: Request, _parent: Inode, _name: &OsStr) -> Result<()> {
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
    }

    async fn rmdir(&self, _req: Request, _parent: Inode, _name: &OsStr) -> Result<()> {
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
    }

    async fn rename(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _new_parent: Inode,
        _new_name: &OsStr,
    ) -> Result<()> {
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
    }

    async fn mkdir(
        &self,
        _req: Request,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
    ) -> Result<ReplyEntry> {
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
    }

    async fn link(
        &self,
        _req: Request,
        _inode: Inode,
        _new_parent: Inode,
        _new_name: &OsStr,
    ) -> Result<ReplyEntry> {
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
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
    async fn open(&self, _req: Request, inode: Inode, flags: u32) -> Result<ReplyOpen> {
        // Dicfuse is strictly read-only. Reject open requests that imply write access so that
        // callers (including overlay layers) can reliably trigger copy-up behavior elsewhere.
        let readonly = flags
            & (libc::O_APPEND | libc::O_CREAT | libc::O_TRUNC | libc::O_RDWR | libc::O_WRONLY)
                as u32
            == 0;
        if !readonly {
            return Err(libc::EROFS.into());
        }

        tracing::debug!("dicfuse: open inode {} (read-only)", inode);
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

        // The file content may be:
        // - cached in-memory (open_buff),
        // - persisted on disk (content.db) but not cached (open_buff bounded/disabled),
        // - not present locally and needs on-demand fetch.
        let mut persisted: Option<Vec<u8>> = None;

        for attempt in 0..2 {
            // Prefer in-memory.
            if let Some(datas) = self.store.get_file_content(inode) {
                let is_empty = datas.is_empty();
                drop(datas);

                if is_empty && attempt == 0 {
                    // Defensive: avoid serving poisoned empty cache for non-empty blobs.
                    let item = self.store.get_inode(inode).await?;
                    if !item.is_dir() && !item.hash.is_empty() && item.hash != EMPTY_BLOB_OID {
                        let _ = self.store.remove_file_by_node(inode);
                        if let Err(e) = self.store.fetch_file_content(inode, &item.hash).await {
                            tracing::warn!(
                                "dicfuse: refetch failed for inode {} oid {}: {}",
                                inode,
                                item.hash,
                                e
                            );
                            let errno = if e.kind() == std::io::ErrorKind::NotFound {
                                libc::ENOENT
                            } else {
                                libc::EIO
                            };
                            return Err(std::io::Error::from_raw_os_error(errno).into());
                        }
                        continue;
                    }
                }

                let datas = self
                    .store
                    .get_file_content(inode)
                    .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
                let _offset = offset as usize;
                let end = (_offset + size as usize).min(datas.len());
                let slice = &datas[_offset..end];
                return Ok(ReplyData {
                    data: Bytes::copy_from_slice(slice),
                });
            }

            // Next: try persisted content.db (without forcing it into open_buff).
            if persisted.is_none() {
                match self.store.get_persisted_file_content(inode) {
                    Ok(v) => persisted = Some(v),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => {
                        tracing::warn!(
                            "dicfuse: failed to read persisted content inode {}: {}",
                            inode,
                            e
                        );
                        return Err(std::io::Error::from_raw_os_error(libc::EIO).into());
                    }
                }
            }

            if let Some(buf) = persisted.as_ref() {
                let is_empty = buf.is_empty();
                if is_empty && attempt == 0 {
                    let item = self.store.get_inode(inode).await?;
                    if !item.is_dir() && !item.hash.is_empty() && item.hash != EMPTY_BLOB_OID {
                        let _ = self.store.remove_file_by_node(inode);
                        if let Err(e) = self.store.fetch_file_content(inode, &item.hash).await {
                            tracing::warn!(
                                "dicfuse: refetch failed for inode {} oid {}: {}",
                                inode,
                                item.hash,
                                e
                            );
                            let errno = if e.kind() == std::io::ErrorKind::NotFound {
                                libc::ENOENT
                            } else {
                                libc::EIO
                            };
                            return Err(std::io::Error::from_raw_os_error(errno).into());
                        }
                        persisted = None;
                        continue;
                    }
                }

                let _offset = offset as usize;
                let end = (_offset + size as usize).min(buf.len());
                let slice = &buf[_offset..end];
                return Ok(ReplyData {
                    data: Bytes::copy_from_slice(slice),
                });
            }

            // Finally: on-demand fetch.
            let item = self.store.get_inode(inode).await?;
            if item.is_dir() {
                return Err(std::io::Error::from_raw_os_error(libc::EISDIR).into());
            }
            if item.hash.is_empty() {
                return Err(std::io::Error::from_raw_os_error(libc::ENOENT).into());
            }
            if let Err(e) = self.store.fetch_file_content(inode, &item.hash).await {
                tracing::warn!(
                    "dicfuse: failed to fetch inode {} oid {}: {}",
                    inode,
                    item.hash,
                    e
                );
                let errno = if e.kind() == std::io::ErrorKind::NotFound {
                    libc::ENOENT
                } else {
                    libc::EIO
                };
                return Err(std::io::Error::from_raw_os_error(errno).into());
            }
        }

        Err(std::io::Error::from_raw_os_error(libc::EIO).into())
    }
    async fn access(&self, _req: Request, inode: Inode, _mask: u32) -> Result<()> {
        // Access is a metadata permission check; keep it lightweight.
        // For directories, ensure at least one children listing exists (lazy).
        let item = self.store.get_inode(inode).await?;
        if item.is_dir() {
            self.store.ensure_dir_loaded(inode).await?;
        }
        Ok(())
    }

    async fn write(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _offset: u64,
        _data: &[u8],
        _write_flags: u32,
        _flags: u32,
    ) -> Result<ReplyWrite> {
        // Read-only filesystem: deny writes.
        Err(libc::EROFS.into())
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
        // Ensure directory entries exist before listing.
        self.store.ensure_dir_loaded(parent).await?;
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

        // NOTE: Avoid any synchronous network IO here.
        // Directory listing must be fast; file contents will be fetched lazily on read().

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
        // Ensure directory entries exist before listing (first access may require one network fetch).
        self.store.ensure_dir_loaded(parent).await?;
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
                // Add timeout to prevent blocking if get_stat or get_filetype hang
                // This can happen if Dicfuse is still loading data or if there's a lock contention
                let item_name = item.get_name();
                // IMPORTANT: avoid triggering network IO in readdirplus.
                // Use a fast local stat (size may be 0 until getattr/open triggers size fetch).
                let get_stat_future = self.get_stat_fast(item.clone());
                let get_filetype_future = item.get_filetype();

                // Use timeout to prevent infinite blocking
                // This is critical when Dicfuse is still loading data in the background
                let (stat_result, filetype) = match tokio::time::timeout(
                    tokio::time::Duration::from_millis(500),
                    async {
                        let stat = get_stat_future.await;
                        let ft = get_filetype_future.await;
                        (stat, ft)
                    },
                )
                .await
                {
                    Ok((stat, ft)) => (stat, ft),
                    Err(_) => {
                        tracing::warn!(
                            "get_stat/get_filetype timed out for item {} in readdirplus (Dicfuse may still be loading)",
                            item_name
                        );
                        // Use default entries to avoid blocking
                        // Try to determine if it's a directory by checking if we can get the item
                        let mut default_entry = match self.store.get_inode(item.get_inode()).await {
                            Ok(i) if i.is_dir() => default_dic_entry(item.get_inode()),
                            _ => default_file_entry(item.get_inode()),
                        };
                        default_entry.ttl = self.reply_ttl();
                        let default_ft = if default_entry.attr.kind == rfuse3::FileType::Directory {
                            rfuse3::FileType::Directory
                        } else {
                            rfuse3::FileType::RegularFile
                        };
                        (default_entry, default_ft)
                    }
                };

                d.push(Ok(DirectoryEntryPlus {
                    inode: item.get_inode(),
                    kind: filetype,
                    name: item_name.into(),
                    offset: entry_offset + 1,
                    generation: 0,
                    attr: stat_result.attr,
                    entry_ttl: stat_result.ttl,
                    attr_ttl: stat_result.ttl,
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
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
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
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
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
        // Read-only filesystem: deny mutation.
        Err(libc::EROFS.into())
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
        // Read-only filesystem: deny writes.
        Err(libc::EROFS.into())
    }

    async fn getlk(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _type: u32,
        _pid: u32,
    ) -> Result<ReplyLock> {
        Err(libc::ENOSYS.into())
    }

    async fn setlk(
        &self,
        _req: Request,
        _inode: Inode,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _type: u32,
        _pid: u32,
        _block: bool,
    ) -> Result<()> {
        Err(libc::ENOSYS.into())
    }
}
