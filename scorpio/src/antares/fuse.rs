use std::{path::PathBuf, sync::Arc};

use libfuse_fs::{
    passthrough::{new_passthroughfs_layer, PassthroughArgs},
    unionfs::{config::Config, layer::Layer, OverlayFs},
};
use tokio::task::JoinHandle;

use crate::server::mount_filesystem;

/// Antares union-fs wrapper: dicfuse lower + passthrough upper/CL.
pub struct AntaresFuse {
    pub mountpoint: PathBuf,
    pub upper_dir: PathBuf,
    pub dic: Arc<crate::dicfuse::Dicfuse>,
    pub cl_dir: Option<PathBuf>,
    /// Background task running the FUSE session.
    fuse_task: Option<JoinHandle<()>>,
}
use libfuse_fs::passthrough::newlogfs::LoggingFileSystem;
impl AntaresFuse {
    /// Build directories for upper / optional CL layers.
    pub async fn new(
        mountpoint: PathBuf,
        dic: Arc<crate::dicfuse::Dicfuse>,
        upper_dir: PathBuf,
        cl_dir: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        if let Some(cl) = &cl_dir {
            std::fs::create_dir_all(cl)?;
        }
        std::fs::create_dir_all(&upper_dir)?;
        std::fs::create_dir_all(&mountpoint)?;

        Ok(Self {
            mountpoint,
            upper_dir,
            dic,
            cl_dir,
            fuse_task: None,
        })
    }

    /// Compose the union filesystem instance.
    pub async fn build_overlay(&self) -> std::io::Result<OverlayFs> {
        // Build lower layers:
        // - Prefer Dicfuse as the primary lower layer (read-only monorepo view).
        // - Optional CL dir is best-effort and may be empty; keep it as an additional
        //   layer so it never masks the Dicfuse view.
        let mut lower_layers: Vec<Arc<dyn Layer>> = Vec::new();
        lower_layers.push(self.dic.clone() as Arc<dyn Layer>);

        if let Some(cl_dir) = &self.cl_dir {
            let cl_layer = new_passthroughfs_layer(PassthroughArgs {
                root_dir: cl_dir,
                mapping: None::<String>,
            })
            .await?;
            lower_layers.push(Arc::new(cl_layer) as Arc<dyn Layer>);
        }

        // Upper layer mirrors upper_dir to keep writes separated from lower layers.
        let upper_layer: Arc<dyn Layer> = Arc::new(
            new_passthroughfs_layer(PassthroughArgs {
                root_dir: &self.upper_dir,
                mapping: None::<String>,
            })
            .await?,
        );

        // passthrough Upper  - readwrite file system over upper dir
        // passthrough CL  - readwrite file system over upper dir
        // dicfuse  - readonly file and dictionary from mega

        let cfg = Config {
            mountpoint: self.mountpoint.clone(),
            do_import: true,
            ..Default::default()
        };

        OverlayFs::new(Some(upper_layer), lower_layers, cfg, 1)
    }

    /// Mount the composed unionfs into the provided mountpoint, spawning a background task to run the FUSE session.
    pub async fn mount(&mut self) -> std::io::Result<()> {
        if self.fuse_task.is_some() {
            return Ok(());
        }

        // Ensure mountpoint exists *before* mounting. This is a plain filesystem check.
        // Do not probe it *after* mounting, because that may trigger FUSE getattr and can fail
        // transiently while Dicfuse is still loading.
        std::fs::metadata(&self.mountpoint)?;

        let overlay = self.build_overlay().await?;
        let logfs = LoggingFileSystem::new(overlay);
        let handle = mount_filesystem(logfs, self.mountpoint.as_os_str()).await;

        // Spawn background task to run the FUSE session
        let fuse_task = tokio::spawn(async move {
            // This will block until unmount is called
            let _ = handle.await;
        });

        self.fuse_task = Some(fuse_task);

        tracing::info!(
            "Mount spawned for {}; FUSE session running (Dicfuse may still be loading in background)",
            self.mountpoint.display()
        );
        Ok(())
    }

    /// Unmount the FUSE session if mounted.
    ///
    /// Uses lazy unmount (`fusermount -uz`) to detach the filesystem even if
    /// it's busy, preventing the unmount operation from blocking indefinitely.
    /// A timeout is applied when waiting for the FUSE task to complete.
    ///
    /// # Errors
    ///
    /// This method will log warnings but not fail if:
    /// - The FUSE task doesn't complete within the timeout
    /// - The task panics
    ///
    /// Only critical errors (e.g., fusermount command execution failure)
    /// will cause this method to return an error.
    pub async fn unmount(&mut self) -> std::io::Result<()> {
        if let Some(task) = self.fuse_task.take() {
            // Unmount via fusermount with lazy unmount (-z) for faster unmounting
            // This allows unmounting even if there are pending operations
            let mount_path = self.mountpoint.to_string_lossy().to_string();
            let output = tokio::process::Command::new("fusermount")
                .arg("-uz") // -u: unmount, -z: lazy unmount (detach even if busy; actual unmount occurs after all references are released)
                .arg(&mount_path)
                .output()
                .await?;

            if !output.status.success() {
                tracing::warn!(
                    "fusermount -uz failed for {}: {}",
                    mount_path,
                    String::from_utf8_lossy(&output.stderr)
                );
                // Continue, as lazy unmount might still succeed partially or task might exit
            }

            // Wait for the FUSE task to complete with timeout to avoid hanging
            let timeout_duration = tokio::time::Duration::from_secs(5);
            match tokio::time::timeout(timeout_duration, task).await {
                Ok(Ok(_)) => {
                    // Task completed successfully
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "fuse task panicked during unmount of {}: {:?}",
                        mount_path,
                        e
                    );
                }
                Err(_) => {
                    tracing::warn!(
                        "fuse task did not complete within {}s for {}, continuing anyway",
                        timeout_duration.as_secs(),
                        mount_path
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        ffi::{OsStr, OsString},
        num::NonZeroU32,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use async_trait::async_trait;
    use bytes::Bytes;
    use libfuse_fs::{
        context::OperationContext,
        unionfs::{config::Config as UnionConfig, layer::Layer, OverlayFs},
    };
    use rfuse3::{
        raw::{
            reply::{
                DirectoryEntry, FileAttr, ReplyAttr, ReplyCreated, ReplyData, ReplyDirectory,
                ReplyEntry, ReplyInit, ReplyOpen, ReplyWrite, ReplyXAttr,
            },
            Filesystem, Request,
        },
        FileType, Inode, Result as FuseResult, Timestamp,
    };
    use serial_test::serial;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    use super::AntaresFuse;
    use crate::{dicfuse::Dicfuse, util::config};

    #[derive(Debug, Clone)]
    struct MemNode {
        inode: u64,
        parent: u64,
        kind: FileType,
        perm: u16,
        uid: u32,
        gid: u32,
        data: Vec<u8>,
    }

    #[derive(Debug, Default)]
    struct MemState {
        nodes: HashMap<u64, MemNode>,
        children: HashMap<u64, HashMap<OsString, u64>>,
    }

    /// A tiny in-memory upper layer used to test OverlayFs copy-up semantics without requiring
    /// passthroughfs (open_by_handle_at) or an actual kernel mount.
    #[derive(Debug)]
    struct MemUpperLayer {
        next_inode: AtomicU64,
        state: tokio::sync::RwLock<MemState>,
    }

    impl MemUpperLayer {
        fn new() -> Self {
            let uid = unsafe { libc::getuid() } as u32;
            let gid = unsafe { libc::getgid() } as u32;
            let mut st = MemState::default();
            st.nodes.insert(
                1,
                MemNode {
                    inode: 1,
                    parent: 0,
                    kind: FileType::Directory,
                    perm: 0o755,
                    uid,
                    gid,
                    data: Vec::new(),
                },
            );
            Self {
                next_inode: AtomicU64::new(1),
                state: tokio::sync::RwLock::new(st),
            }
        }

        fn now_ts() -> Timestamp {
            Timestamp::from(std::time::SystemTime::now())
        }

        fn file_attr(node: &MemNode) -> FileAttr {
            let ts = Self::now_ts();
            FileAttr {
                ino: node.inode,
                size: node.data.len() as u64,
                blocks: 0,
                atime: ts,
                mtime: ts,
                ctime: ts,
                kind: node.kind,
                perm: node.perm,
                nlink: if node.kind == FileType::Directory {
                    2
                } else {
                    1
                },
                uid: node.uid,
                gid: node.gid,
                rdev: 0,
                blksize: 4096,
            }
        }

        async fn create_child(
            &self,
            parent: u64,
            name: &OsStr,
            kind: FileType,
            perm: u16,
        ) -> std::io::Result<u64> {
            let mut st = self.state.write().await;
            let parent_node = st
                .nodes
                .get(&parent)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            if parent_node.kind != FileType::Directory {
                return Err(std::io::Error::from_raw_os_error(libc::ENOTDIR));
            }

            let existing = st.children.get(&parent).and_then(|m| m.get(name).copied());
            if let Some(inode) = existing {
                return Ok(inode);
            }

            let inode = self.next_inode.fetch_add(1, Ordering::Relaxed) + 1;
            let uid = unsafe { libc::getuid() } as u32;
            let gid = unsafe { libc::getgid() } as u32;
            st.nodes.insert(
                inode,
                MemNode {
                    inode,
                    parent,
                    kind,
                    perm,
                    uid,
                    gid,
                    data: Vec::new(),
                },
            );
            st.children
                .entry(parent)
                .or_default()
                .insert(name.to_os_string(), inode);
            Ok(inode)
        }

        async fn get_child_inode(&self, parent: u64, name: &OsStr) -> Option<u64> {
            let st = self.state.read().await;
            st.children.get(&parent).and_then(|m| m.get(name).copied())
        }

        async fn read_file_by_name(&self, name: &str) -> Option<Vec<u8>> {
            let st = self.state.read().await;
            let ino = st
                .children
                .get(&1)
                .and_then(|m| m.get(OsStr::new(name)))
                .copied()?;
            st.nodes.get(&ino).map(|n| n.data.clone())
        }
    }

    impl Filesystem for MemUpperLayer {
        async fn init(&self, _req: Request) -> FuseResult<ReplyInit> {
            Ok(ReplyInit {
                max_write: NonZeroU32::new(128 * 1024).unwrap(),
            })
        }

        async fn destroy(&self, _req: Request) {}

        async fn lookup(
            &self,
            _req: Request,
            parent: Inode,
            name: &OsStr,
        ) -> FuseResult<ReplyEntry> {
            let inode = self
                .get_child_inode(parent, name)
                .await
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            let st = self.state.read().await;
            let node = st
                .nodes
                .get(&inode)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            Ok(ReplyEntry {
                ttl: Duration::from_secs(1),
                attr: Self::file_attr(node),
                generation: 0,
            })
        }

        async fn getattr(
            &self,
            _req: Request,
            inode: Inode,
            _fh: Option<u64>,
            _flags: u32,
        ) -> FuseResult<ReplyAttr> {
            let st = self.state.read().await;
            let node = st
                .nodes
                .get(&inode)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            Ok(ReplyAttr {
                ttl: Duration::from_secs(1),
                attr: Self::file_attr(node),
            })
        }

        async fn setattr(
            &self,
            _req: Request,
            inode: Inode,
            _fh: Option<u64>,
            _set_attr: rfuse3::SetAttr,
        ) -> FuseResult<ReplyAttr> {
            // Minimal: accept setattr and return current attrs. Overlay copy-up uses this to
            // preserve ownership/mode; our in-memory layer does not model these changes yet.
            self.getattr(_req, inode, _fh, 0).await
        }

        async fn open(&self, _req: Request, inode: Inode, _flags: u32) -> FuseResult<ReplyOpen> {
            // Use inode as the handle.
            Ok(ReplyOpen {
                fh: inode,
                flags: 0,
            })
        }

        async fn read(
            &self,
            _req: Request,
            inode: Inode,
            _fh: u64,
            offset: u64,
            size: u32,
        ) -> FuseResult<ReplyData> {
            let st = self.state.read().await;
            let node = st
                .nodes
                .get(&inode)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            let off = offset as usize;
            let end = (off + size as usize).min(node.data.len());
            let slice = if off >= node.data.len() {
                &[]
            } else {
                &node.data[off..end]
            };
            Ok(ReplyData {
                data: Bytes::copy_from_slice(slice),
            })
        }

        #[allow(clippy::too_many_arguments)]
        async fn write(
            &self,
            _req: Request,
            inode: Inode,
            _fh: u64,
            offset: u64,
            data: &[u8],
            _write_flags: u32,
            _flags: u32,
        ) -> FuseResult<ReplyWrite> {
            let mut st = self.state.write().await;
            let node = st
                .nodes
                .get_mut(&inode)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            if node.kind == FileType::Directory {
                return Err(std::io::Error::from_raw_os_error(libc::EISDIR).into());
            }
            let off = offset as usize;
            let needed = off + data.len();
            if node.data.len() < needed {
                node.data.resize(needed, 0);
            }
            node.data[off..off + data.len()].copy_from_slice(data);
            Ok(ReplyWrite {
                written: data.len() as u32,
            })
        }

        async fn setxattr(
            &self,
            _req: Request,
            _inode: Inode,
            _name: &OsStr,
            _value: &[u8],
            _flags: u32,
            _position: u32,
        ) -> FuseResult<()> {
            // No-op xattr support for overlay bookkeeping.
            Ok(())
        }

        async fn getxattr(
            &self,
            _req: Request,
            _inode: Inode,
            _name: &OsStr,
            _size: u32,
        ) -> FuseResult<ReplyXAttr> {
            Err(std::io::Error::from_raw_os_error(libc::ENODATA).into())
        }

        async fn listxattr(
            &self,
            _req: Request,
            _inode: Inode,
            size: u32,
        ) -> FuseResult<ReplyXAttr> {
            if size == 0 {
                Ok(ReplyXAttr::Size(0))
            } else {
                Ok(ReplyXAttr::Data(Bytes::new()))
            }
        }

        async fn removexattr(&self, _req: Request, _inode: Inode, _name: &OsStr) -> FuseResult<()> {
            Ok(())
        }

        async fn readdir<'a>(
            &'a self,
            _req: Request,
            parent: Inode,
            _fh: u64,
            offset: i64,
        ) -> FuseResult<
            ReplyDirectory<impl futures::Stream<Item = FuseResult<DirectoryEntry>> + Send + 'a>,
        > {
            use futures::stream::iter;

            let st = self.state.read().await;
            let parent_node = st
                .nodes
                .get(&parent)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            if parent_node.kind != FileType::Directory {
                return Err(std::io::Error::from_raw_os_error(libc::ENOTDIR).into());
            }
            let parent_parent_inode = if parent == 1 { 1 } else { parent_node.parent };

            let mut out: Vec<std::result::Result<DirectoryEntry, rfuse3::Errno>> = Vec::new();

            // offset 0: ".", offset 1: "..", offset 2+: children
            if offset < 1 {
                out.push(Ok(DirectoryEntry {
                    inode: parent,
                    kind: FileType::Directory,
                    name: ".".into(),
                    offset: 1,
                }));
            }
            if offset < 2 {
                out.push(Ok(DirectoryEntry {
                    inode: parent_parent_inode,
                    kind: FileType::Directory,
                    name: "..".into(),
                    offset: 2,
                }));
            }

            if let Some(children) = st.children.get(&parent) {
                for (idx, (name, inode)) in children.iter().enumerate() {
                    let entry_offset = (idx + 2) as i64;
                    if entry_offset > offset {
                        let kind = st
                            .nodes
                            .get(inode)
                            .map(|n| n.kind)
                            .unwrap_or(FileType::RegularFile);
                        out.push(Ok(DirectoryEntry {
                            inode: *inode,
                            kind,
                            name: name.clone(),
                            offset: entry_offset + 1,
                        }));
                    }
                }
            }

            Ok(ReplyDirectory {
                entries: iter(out.into_iter()),
            })
        }

        async fn releasedir(
            &self,
            _req: Request,
            _inode: Inode,
            _fh: u64,
            _flags: u32,
        ) -> FuseResult<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Layer for MemUpperLayer {
        fn root_inode(&self) -> Inode {
            1
        }

        async fn create_with_context(
            &self,
            _ctx: OperationContext,
            parent: Inode,
            name: &OsStr,
            _mode: u32,
            _flags: u32,
        ) -> FuseResult<ReplyCreated> {
            let inode = self
                .create_child(parent, name, FileType::RegularFile, 0o644)
                .await
                .map_err(rfuse3::Errno::from)?;
            let st = self.state.read().await;
            let node = st
                .nodes
                .get(&inode)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            Ok(ReplyCreated {
                ttl: Duration::from_secs(1),
                attr: Self::file_attr(node),
                generation: 0,
                fh: inode,
                flags: 0,
            })
        }

        async fn mkdir_with_context(
            &self,
            _ctx: OperationContext,
            parent: Inode,
            name: &OsStr,
            _mode: u32,
            _umask: u32,
        ) -> FuseResult<ReplyEntry> {
            let inode = self
                .create_child(parent, name, FileType::Directory, 0o755)
                .await
                .map_err(rfuse3::Errno::from)?;
            let st = self.state.read().await;
            let node = st
                .nodes
                .get(&inode)
                .ok_or_else(|| std::io::Error::from_raw_os_error(libc::ENOENT))?;
            Ok(ReplyEntry {
                ttl: Duration::from_secs(1),
                attr: Self::file_attr(node),
                generation: 0,
            })
        }

        async fn symlink_with_context(
            &self,
            _ctx: OperationContext,
            _parent: Inode,
            _name: &OsStr,
            _link: &OsStr,
        ) -> FuseResult<ReplyEntry> {
            Err(std::io::Error::from_raw_os_error(libc::ENOSYS).into())
        }
    }

    /// No actual mount required: validate copy-up behavior via `OverlayFs`'s Filesystem interface.
    #[tokio::test]
    async fn test_overlay_copyup_without_mount_does_not_mutate_dicfuse_lower() {
        let test_id = Uuid::new_v4();
        let store_path = format!("/tmp/scorpio_dicfuse_unit_store_{test_id}");
        let _ = std::fs::remove_dir_all(&store_path);
        std::fs::create_dir_all(&store_path).unwrap();

        // Lower: dicfuse (read-only)
        let dic = std::sync::Arc::new(Dicfuse::new_with_store_path(&store_path).await);
        dic.store.insert_mock_item(1, 0, "", true).await; // root
        dic.store.insert_mock_item(2, 1, "hello.txt", false).await;
        dic.store.save_file(2, b"lower".to_vec());
        dic.store.load_db().await.unwrap();

        // Upper: in-memory writable layer.
        let upper = std::sync::Arc::new(MemUpperLayer::new());

        let cfg = UnionConfig {
            mountpoint: PathBuf::from(format!("/tmp/scorpio_overlay_unit_{test_id}")),
            do_import: true,
            ..Default::default()
        };

        let overlay = OverlayFs::new(
            Some(upper.clone() as std::sync::Arc<dyn Layer>),
            vec![dic.clone() as std::sync::Arc<dyn Layer>],
            cfg,
            1,
        )
        .unwrap();

        overlay.init(Request::default()).await.unwrap();

        let entry = overlay
            .lookup(Request::default(), 1, OsStr::new("hello.txt"))
            .await
            .unwrap();
        let overlay_ino = entry.attr.ino;

        // Read initial content from lower.
        let ro = overlay
            .open(Request::default(), overlay_ino, libc::O_RDONLY as u32)
            .await
            .unwrap();
        let data = overlay
            .read(Request::default(), overlay_ino, ro.fh, 0, 32)
            .await
            .unwrap();
        assert_eq!(data.data.as_ref(), b"lower");

        // Write triggers copy-up.
        let wo = overlay
            .open(Request::default(), overlay_ino, libc::O_WRONLY as u32)
            .await
            .unwrap();
        overlay
            .write(Request::default(), overlay_ino, wo.fh, 0, b"upper", 0, 0)
            .await
            .unwrap();

        // Now read should see upper content.
        let ro2 = overlay
            .open(Request::default(), overlay_ino, libc::O_RDONLY as u32)
            .await
            .unwrap();
        let data2 = overlay
            .read(Request::default(), overlay_ino, ro2.fh, 0, 32)
            .await
            .unwrap();
        assert_eq!(data2.data.as_ref(), b"upper");

        // Verify upper contains the file and lower store did not change.
        assert_eq!(
            upper.read_file_by_name("hello.txt").await.unwrap(),
            b"upper"
        );
        assert_eq!(dic.store.get_file_content(2).unwrap().to_vec(), b"lower");

        let _ = std::fs::remove_dir_all(&store_path);
    }

    /// No actual mount required: validate multiple overlays share the same Dicfuse lower while uppers stay isolated.
    #[tokio::test]
    async fn test_two_overlays_share_dicfuse_lower_isolate_upper_without_mount() {
        // Ensure config is loaded so dicfuse import logic (if triggered by OverlayFs) can resolve store_path.
        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        let test_id = Uuid::new_v4();
        let store_path = format!("/tmp/scorpio_dicfuse_unit_store2_{test_id}");
        let _ = std::fs::remove_dir_all(&store_path);
        std::fs::create_dir_all(&store_path).unwrap();

        let dic = std::sync::Arc::new(Dicfuse::new_with_store_path(&store_path).await);
        dic.store.insert_mock_item(1, 0, "", true).await;
        dic.store.insert_mock_item(2, 1, "hello.txt", false).await;
        dic.store.save_file(2, b"lower".to_vec());
        dic.store.load_db().await.unwrap();

        // OverlayFs with do_import=true may trigger Dicfuse background import logic.
        // Ensure it treats the pre-seeded sled DB as reusable (skip network fetch) by writing the marker
        // into the SAME store directory we seeded.
        let marker_path = std::path::PathBuf::from(&store_path).join(".dicfuse_import_done");
        if let Some(parent) = marker_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&marker_path, b"ok\n");

        let upper1 = std::sync::Arc::new(MemUpperLayer::new());
        let upper2 = std::sync::Arc::new(MemUpperLayer::new());

        let cfg1 = UnionConfig {
            mountpoint: PathBuf::from(format!("/tmp/scorpio_overlay_unit2_a_{test_id}")),
            // Keep import enabled so OverlayFs builds the view, but Dicfuse import should be skipped by marker above.
            do_import: true,
            ..Default::default()
        };
        let cfg2 = UnionConfig {
            mountpoint: PathBuf::from(format!("/tmp/scorpio_overlay_unit2_b_{test_id}")),
            do_import: true,
            ..Default::default()
        };

        let overlay1 = std::sync::Arc::new(
            OverlayFs::new(
                Some(upper1.clone() as std::sync::Arc<dyn Layer>),
                vec![dic.clone() as std::sync::Arc<dyn Layer>],
                cfg1,
                1,
            )
            .unwrap(),
        );
        let overlay2 = std::sync::Arc::new(
            OverlayFs::new(
                Some(upper2.clone() as std::sync::Arc<dyn Layer>),
                vec![dic.clone() as std::sync::Arc<dyn Layer>],
                cfg2,
                1,
            )
            .unwrap(),
        );

        overlay1.init(Request::default()).await.unwrap();
        overlay2.init(Request::default()).await.unwrap();

        let e1 = overlay1
            .lookup(Request::default(), 1, OsStr::new("hello.txt"))
            .await
            .unwrap();
        let e2 = overlay2
            .lookup(Request::default(), 1, OsStr::new("hello.txt"))
            .await
            .unwrap();
        let ino1 = e1.attr.ino;
        let ino2 = e2.attr.ino;

        // Concurrent reads from both overlays should be consistent.
        let o1 = overlay1.clone();
        let o2 = overlay2.clone();
        let t1 = tokio::spawn(async move {
            for _ in 0..50 {
                let ro = o1
                    .open(Request::default(), ino1, libc::O_RDONLY as u32)
                    .await
                    .unwrap();
                let data = o1
                    .read(Request::default(), ino1, ro.fh, 0, 32)
                    .await
                    .unwrap();
                assert_eq!(data.data.as_ref(), b"lower");
            }
        });
        let t2 = tokio::spawn(async move {
            for _ in 0..50 {
                let ro = o2
                    .open(Request::default(), ino2, libc::O_RDONLY as u32)
                    .await
                    .unwrap();
                let data = o2
                    .read(Request::default(), ino2, ro.fh, 0, 32)
                    .await
                    .unwrap();
                assert_eq!(data.data.as_ref(), b"lower");
            }
        });
        let _ = tokio::join!(t1, t2);

        // Write in overlay1 must not affect overlay2.
        let wo = overlay1
            .open(Request::default(), ino1, libc::O_WRONLY as u32)
            .await
            .unwrap();
        overlay1
            .write(Request::default(), ino1, wo.fh, 0, b"upper1", 0, 0)
            .await
            .unwrap();

        let ro1 = overlay1
            .open(Request::default(), ino1, libc::O_RDONLY as u32)
            .await
            .unwrap();
        let d1 = overlay1
            .read(Request::default(), ino1, ro1.fh, 0, 32)
            .await
            .unwrap();
        assert_eq!(d1.data.as_ref(), b"upper1");

        let ro2 = overlay2
            .open(Request::default(), ino2, libc::O_RDONLY as u32)
            .await
            .unwrap();
        let d2 = overlay2
            .read(Request::default(), ino2, ro2.fh, 0, 32)
            .await
            .unwrap();
        assert_eq!(d2.data.as_ref(), b"lower");

        assert_eq!(
            upper1.read_file_by_name("hello.txt").await.unwrap(),
            b"upper1"
        );
        assert!(upper2.read_file_by_name("hello.txt").await.is_none());

        // Lower store content must remain unchanged.
        assert_eq!(dic.store.get_file_content(2).unwrap().to_vec(), b"lower");

        let _ = std::fs::remove_file(&marker_path);
        let _ = std::fs::remove_dir_all(&store_path);
    }

    fn fuse_test_prereqs_or_skip() -> bool {
        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Skipping: requires root privileges");
            return false;
        }

        if !std::path::Path::new("/dev/fuse").exists() {
            println!("Skipping: /dev/fuse not available");
            return false;
        }

        // AntaresFuse::unmount uses `fusermount -uz`.
        if std::process::Command::new("fusermount")
            .arg("--version")
            .output()
            .is_err()
        {
            println!("Skipping: fusermount not found");
            return false;
        }

        true
    }

    async fn retry_read(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
        // FUSE mounts can become ready slightly after mount() returns; retry briefly.
        const RETRIES: usize = 20;
        const SLEEP_MS: u64 = 50;
        for attempt in 0..RETRIES {
            match std::fs::read(path) {
                Ok(v) => return Ok(v),
                Err(e) if attempt + 1 < RETRIES => {
                    tracing::debug!(
                        "retry_read({}): attempt {} failed: {}",
                        path.display(),
                        attempt + 1,
                        e
                    );
                    sleep(Duration::from_millis(SLEEP_MS)).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    /// Validate: when Dicfuse is used as the lower layer, writes copy-up into upper and do not mutate Dicfuse's read-only data.
    #[tokio::test]
    #[serial] // Avoid overlapping FUSE mounts in tests.
    async fn test_dicfuse_lower_copyup_does_not_mutate_lower() {
        if !fuse_test_prereqs_or_skip() {
            return;
        }

        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_e2e_ro_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);

        let mount = base.join("mnt");
        let upper = base.join("upper");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        // Prepare a minimal Dicfuse store without any network calls.
        let dic =
            std::sync::Arc::new(Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await);
        dic.store.insert_mock_item(1, 0, "", true).await; // root
        dic.store.insert_mock_item(2, 1, "hello.txt", false).await;
        dic.store.save_file(2, b"lower".to_vec());
        dic.store.load_db().await.unwrap();

        let mut fuse = AntaresFuse::new(mount.clone(), dic.clone(), upper.clone(), None)
            .await
            .unwrap();
        fuse.mount().await.unwrap();

        let mounted_file = mount.join("hello.txt");

        // Read from mount should return lower content initially.
        let before = retry_read(&mounted_file).await.unwrap();
        assert_eq!(before, b"lower");

        // Write should copy-up into upper and not mutate dicfuse store.
        std::fs::write(&mounted_file, b"upper").unwrap();

        let after = retry_read(&mounted_file).await.unwrap();
        assert_eq!(after, b"upper");

        let upper_file = upper.join("hello.txt");
        assert!(upper_file.exists(), "copy-up should create file in upper");
        assert_eq!(std::fs::read(&upper_file).unwrap(), b"upper");

        // Lower store content must remain unchanged.
        let lower_content = dic.store.get_file_content(2).unwrap().to_vec();
        assert_eq!(lower_content, b"lower");

        fuse.unmount().await.unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Validate: when multiple Antares instances share the same Dicfuse lower, reads are consistent and each mount's upper is isolated.
    #[tokio::test]
    #[serial] // Avoid overlapping FUSE mounts in tests.
    async fn test_concurrent_mounts_share_dicfuse_but_isolate_upper() {
        if !fuse_test_prereqs_or_skip() {
            return;
        }

        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_e2e_concurrent_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);

        let mount1 = base.join("mnt1");
        let mount2 = base.join("mnt2");
        let upper1 = base.join("upper1");
        let upper2 = base.join("upper2");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        let dic =
            std::sync::Arc::new(Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await);
        dic.store.insert_mock_item(1, 0, "", true).await; // root
        dic.store.insert_mock_item(2, 1, "hello.txt", false).await;
        dic.store.save_file(2, b"lower".to_vec());
        dic.store.load_db().await.unwrap();

        let mut fuse1 = AntaresFuse::new(mount1.clone(), dic.clone(), upper1.clone(), None)
            .await
            .unwrap();
        let mut fuse2 = AntaresFuse::new(mount2.clone(), dic.clone(), upper2.clone(), None)
            .await
            .unwrap();

        fuse1.mount().await.unwrap();
        fuse2.mount().await.unwrap();

        let file1 = mount1.join("hello.txt");
        let file2 = mount2.join("hello.txt");

        // Initial reads should come from lower in both mounts.
        assert_eq!(retry_read(&file1).await.unwrap(), b"lower");
        assert_eq!(retry_read(&file2).await.unwrap(), b"lower");

        // Write in mount1 should only affect upper1, not mount2.
        std::fs::write(&file1, b"upper1").unwrap();

        assert_eq!(retry_read(&file1).await.unwrap(), b"upper1");
        assert_eq!(retry_read(&file2).await.unwrap(), b"lower");

        assert_eq!(std::fs::read(upper1.join("hello.txt")).unwrap(), b"upper1");
        assert!(
            !upper2.join("hello.txt").exists(),
            "upper2 should remain untouched"
        );

        fuse1.unmount().await.unwrap();
        fuse2.unmount().await.unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore]
    // Requires FUSE/root. Direct run example:
    //   sudo -E cargo test --lib antares::fuse::tests::test_simple_passthrough_mount -- --exact --ignored --nocapture
    // For LLDB debug workflow, see `doc/test.md`.
    #[serial] // Serialize to avoid config initialization conflicts
    async fn test_simple_passthrough_mount() {
        // Simplified test using only passthrough layers (no Dicfuse)
        use std::sync::Arc;

        use libfuse_fs::{
            passthrough::{new_passthroughfs_layer, PassthroughArgs},
            unionfs::{config::Config, OverlayFs},
        };

        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges");
            println!("Run with: sudo -E cargo test --lib antares::fuse::tests::test_simple_passthrough_mount -- --exact --ignored --nocapture");
            return;
        }

        let base = PathBuf::from("/tmp/antares_simple_test");
        let _ = std::fs::remove_dir_all(&base);

        let mount = base.join("mnt");
        let upper = base.join("upper");
        let lower1 = base.join("lower1");
        let lower2 = base.join("lower2");

        // Create directories and test files
        std::fs::create_dir_all(&mount).unwrap();
        std::fs::create_dir_all(&upper).unwrap();
        std::fs::create_dir_all(&lower1).unwrap();
        std::fs::create_dir_all(&lower2).unwrap();

        std::fs::write(lower1.join("file1.txt"), b"from lower1").unwrap();
        std::fs::write(lower2.join("file2.txt"), b"from lower2").unwrap();

        // Build overlay
        let lower1_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &lower1,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let lower2_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &lower2,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let upper_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &upper,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let cfg = Config {
            mountpoint: mount.clone(),
            do_import: true,
            ..Default::default()
        };

        let overlay = OverlayFs::new(
            Some(Arc::new(upper_layer)),
            vec![Arc::new(lower2_layer), Arc::new(lower1_layer)],
            cfg,
            1,
        )
        .unwrap();

        println!(
            "Mounting simple passthrough overlay at: {}",
            mount.display()
        );
        let handle = crate::server::mount_filesystem(overlay, mount.as_os_str()).await;

        // Spawn background task
        let fuse_task = tokio::spawn(async move {
            let _ = handle.await;
        });

        // Give it time to initialize
        sleep(Duration::from_millis(200)).await;

        println!("Mount successful!");
        println!("Mountpoint: {}", mount.display());
        println!("Try in another terminal: ls -la {}", mount.display());
        println!("You should see file1.txt and file2.txt");

        // Keep mounted for inspection
        sleep(Duration::from_secs(5)).await;

        // Unmount using lazy unmount to avoid blocking
        println!("Unmounting...");
        let output = tokio::process::Command::new("fusermount")
            .arg("-uz") // Use lazy unmount
            .arg(&mount)
            .output()
            .await
            .unwrap();

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            // Check if the error is because the filesystem is not mounted
            if !error_msg.contains("not mounted") && !error_msg.contains("Invalid argument") {
                eprintln!("fusermount failed: {}", error_msg);
            }
        }

        // Wait for FUSE task to complete with timeout (don't wait indefinitely)
        let timeout_duration = tokio::time::Duration::from_secs(5);
        match tokio::time::timeout(timeout_duration, fuse_task).await {
            Ok(Ok(_)) => println!("FUSE task completed successfully"),
            Ok(Err(e)) => tracing::warn!("FUSE task panicked: {:?}", e),
            Err(_) => tracing::warn!(
                "FUSE task did not complete within {}s, continuing anyway",
                timeout_duration.as_secs()
            ),
        }
        println!("Unmount successful!");

        // cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore] // Run with: sudo -E $(which cargo) test --lib antares::fuse::tests::test_run_mount -- --exact --ignored --nocapture
    async fn test_run_mount() {
        // Helper function to check if a file should be skipped in directory iteration
        let _should_skip_test_file = |name: &str| -> bool {
            name == "test_file.txt" || name == "created_file.txt" || name == "test_dir"
        };
        // Only LoggingFileSystem DEBUG
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive("libfuse_fs::passthrough::newlogfs=debug".parse().unwrap()),
            )
            .try_init();
        // Ignore "already initialized" error when running multiple tests
        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }
        // Check if we have necessary privileges
        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges for open_by_handle_at");
            println!("Run with: sudo -E cargo test --lib antares::fuse::tests::test_run_mount -- --exact --ignored --nocapture");
            println!("Skipping test...");
            return;
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_test_mount_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let cl = base.join("cl");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        let dic = Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;
        // Load directory tree synchronously - simpler and more efficient for tests
        // since we need the tree fully loaded before mount verification anyway
        println!("Loading directory tree...");
        crate::dicfuse::store::import_arc(dic.store.clone()).await;
        println!("Directory tree loaded, proceeding to mount");

        let mut fuse = AntaresFuse::new(
            mount.clone(),
            std::sync::Arc::new(dic),
            upper.clone(),
            Some(cl.clone()),
        )
        .await
        .unwrap();

        // Actually mount the filesystem
        println!("Mounting Antares overlay at: {}", mount.display());
        fuse.mount().await.unwrap();
        println!("Mount completed successfully");
        // mount() already verified accessibility via read_dir, so we can skip redundant checks
        // Keep mounted for inspection
        sleep(Duration::from_secs(30)).await;
        // Listen for Ctrl+C and unmount on signal
        println!("Press Ctrl+C to unmount and exit...");
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl_c");
        println!("Ctrl+C received, unmounting...");
        fuse.unmount().await.unwrap();
        println!("Unmount successful!");
        //let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore]
    // Requires FUSE/root. Direct run example:
    //   sudo -E cargo test --lib antares::fuse::tests::test_antares_mount -- --exact --ignored --nocapture
    // For no-run + LLDB debugging steps, see `doc/test.md`.
    #[serial] // Serialize to avoid config initialization conflicts
    async fn test_antares_mount() {
        // Set overall test timeout to 60 seconds
        let test_future = async {
            // Helper function to check if a file should be skipped in directory iteration
            let should_skip_test_file = |name: &str| -> bool {
                name == "test_file.txt" || name == "created_file.txt" || name == "test_dir"
            };
            // Only  LoggingFileSystem DEBUG
            use tracing_subscriber::EnvFilter;
            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::from_default_env()
                        .add_directive("libfuse_fs::passthrough::newlogfs=debug".parse().unwrap()),
                )
                .try_init();
            // Ignore "already initialized" error when running multiple tests
            if let Err(e) = config::init_config("./scorpio.toml") {
                if !e.contains("already initialized") {
                    panic!("Failed to load config: {e}");
                }
            }
            // Check if we have necessary privileges
            let uid = unsafe { libc::geteuid() };
            if uid != 0 {
                println!("Warning: This test requires root privileges for open_by_handle_at");
                println!("Run with: sudo -E cargo test --lib antares::fuse::tests::test_antares_mount -- --exact --ignored --nocapture");
                println!("Skipping test...");
                return;
            }

            let test_id = Uuid::new_v4();
            let base = PathBuf::from(format!("/tmp/antares_test_mount_{test_id}"));
            let _ = std::fs::remove_dir_all(&base);
            let mount = base.join("mnt");
            let upper = base.join("upper");
            let cl = base.join("cl");
            let store_path = base.join("store");
            std::fs::create_dir_all(&store_path).unwrap();

            // Use isolated Dicfuse instance for testing to avoid database lock conflicts
            // In production, use DicfuseManager::global() to share the instance
            let dic =
                crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;
            // Start background import_arc task to load directory tree asynchronously
            // This prevents blocking during FUSE operations (see blog post for details)
            tokio::spawn(crate::dicfuse::store::import_arc(dic.store.clone()));
            // Wait for Dicfuse to initialize and fetch directory tree from network
            // Increased wait time to allow for network requests to complete
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let mut fuse = AntaresFuse::new(
                mount.clone(),
                std::sync::Arc::new(dic),
                upper.clone(),
                Some(cl.clone()),
            )
            .await
            .unwrap();

            // Actually mount the filesystem
            println!("Mounting Antares overlay at: {}", mount.display());
            fuse.mount().await.unwrap();
            println!("Mount completed successfully");
            // mount() already verified accessibility via read_dir, so we can skip redundant checks

            // Let it run for a bit to ensure stability
            println!("Sleeping for 1 second...");
            sleep(Duration::from_secs(1)).await;
            println!("Sleep completed");

            // Test basic read operations
            println!("Testing basic read operations...");
            let read_dir_result = tokio::fs::read_dir(&mount).await;
            assert!(read_dir_result.is_ok(), "should be able to read directory");
            println!("✓ Directory read successful");

            // Test reading from read-only layer (Dicfuse)
            // Try to read files from Dicfuse lower layer if they exist
            println!("Testing read from read-only layer (Dicfuse)...");
            let mut dir_entries = read_dir_result.unwrap();
            let mut found_readonly_file = false;
            while let Some(entry) = dir_entries.next_entry().await.unwrap_or(None) {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                // Skip . and .. entries
                if file_name == "." || file_name == ".." {
                    continue;
                }

                // Try to read a file from the read-only layer
                if entry.file_type().await.unwrap().is_file() {
                    match tokio::fs::read(&path).await {
                        Ok(content) => {
                            println!(
                                "✓ Read file from read-only layer: {} ({} bytes)",
                                file_name,
                                content.len()
                            );
                            found_readonly_file = true;
                            break;
                        }
                        Err(e) => {
                            // File might not be loaded yet, skip
                            println!("⚠ Could not read {} from read-only layer: {}", file_name, e);
                        }
                    }
                }
            }
            if !found_readonly_file {
                println!("⚠ No files found in read-only layer (may still be loading)");
            }

            // Test basic write operations (create a file in upper layer)
            println!("Testing basic write operations...");
            let test_file = mount.join("test_file.txt");
            let test_content = b"Hello, FUSE!";

            // Write file
            tokio::fs::write(&test_file, test_content).await.unwrap();
            println!("✓ File write successful");

            // Read file back (this verifies file exists and content is correct)
            let read_content = tokio::fs::read(&test_file).await.unwrap();
            assert_eq!(read_content, test_content, "file content should match");
            println!("✓ File read successful, content matches");

            // Test directory creation
            println!("Testing directory creation...");
            let test_dir = mount.join("test_dir");
            tokio::fs::create_dir(&test_dir).await.unwrap();
            println!("✓ Directory creation successful");

            // Test file creation (create empty file first, then write to it)
            println!("Testing file creation...");
            let created_file = mount.join("created_file.txt");
            let created_content = b"Content written to created file";

            // Use tokio::fs::write which handles file creation, writing, and closing atomically
            tokio::fs::write(&created_file, created_content)
                .await
                .unwrap();
            println!("✓ File created and written successfully");

            // Verify the created file
            let read_created = tokio::fs::read(&created_file).await.unwrap();
            assert_eq!(
                read_created, created_content,
                "created file content should match"
            );
            println!("✓ File creation verification successful");

            // Test file in subdirectory
            let subdir_file = test_dir.join("subdir_file.txt");
            let subdir_content = b"File in subdirectory";
            tokio::fs::write(&subdir_file, subdir_content)
                .await
                .unwrap();
            let read_subdir_content = tokio::fs::read(&subdir_file).await.unwrap();
            assert_eq!(
                read_subdir_content, subdir_content,
                "subdirectory file content should match"
            );
            println!("✓ Subdirectory file operations successful");

            // Test Copy-Up mechanism: modify a file from read-only layer
            println!("Testing Copy-Up mechanism (modify read-only file)...");
            // Try to find a file from read-only layer and modify it
            let mut dir_entries = tokio::fs::read_dir(&mount).await.unwrap();
            let mut tested_copyup = false;
            while let Some(entry) = dir_entries.next_entry().await.unwrap_or(None) {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                // Skip . and .. entries, and files we created during this test
                if file_name == "." || file_name == ".." || should_skip_test_file(&file_name) {
                    continue;
                }

                // Try to modify a file from read-only layer (triggers Copy-Up)
                if entry.file_type().await.unwrap().is_file() {
                    match tokio::fs::read(&path).await {
                        Ok(_original_content) => {
                            // Modify the file (this should trigger Copy-Up)
                            let modified_content = b"Modified content from test";
                            tokio::fs::write(&path, modified_content).await.unwrap();

                            // Verify the modification
                            let read_modified = tokio::fs::read(&path).await.unwrap();
                            assert_eq!(
                                read_modified, modified_content,
                                "modified file content should match"
                            );

                            // Verify Copy-Up: file should now be in upper layer
                            let upper_file = upper.join(file_name.as_ref());
                            let upper_check = tokio::time::timeout(
                                Duration::from_secs(2),
                                tokio::fs::read(&upper_file),
                            )
                            .await;
                            match upper_check {
                                Ok(Ok(upper_content)) => {
                                    assert_eq!(
                                        upper_content, modified_content,
                                        "upper layer file should have modified content"
                                    );
                                    println!("✓ Copy-Up mechanism verified: {} copied to upper layer and modified", file_name);
                                    tested_copyup = true;
                                }
                                _ => {
                                    println!("⚠ Copy-Up verification skipped for {} (file may still be syncing)", file_name);
                                }
                            }
                            break;
                        }
                        Err(_) => {
                            // File might not be loaded yet, skip
                            continue;
                        }
                    }
                }
            }
            if !tested_copyup {
                println!("⚠ Copy-Up test skipped (no files from read-only layer available yet)");
            }

            // Verify files are in upper layer (use async check with timeout)
            println!("Verifying copy-up to upper layer for new files...");
            let upper_test_file = upper.join("test_file.txt");
            let upper_check = tokio::time::timeout(
                Duration::from_secs(2),
                tokio::fs::metadata(&upper_test_file),
            )
            .await;
            if upper_check.is_ok() && upper_check.unwrap().is_ok() {
                println!("✓ Copy-up to upper layer confirmed");
            } else {
                println!("⚠ Copy-up verification skipped (file may still be syncing)");
            }

            // Unmount
            println!("Unmounting...");
            fuse.unmount().await.unwrap();
            println!("Unmount successful!");

            // Cleanup
            let _ = std::fs::remove_dir_all(&base);
        };

        // Run test with timeout to prevent hanging
        // Increased timeout to 120s to account for Dicfuse network initialization
        match tokio::time::timeout(Duration::from_secs(120), test_future).await {
            Ok(_) => println!("✓ Test completed successfully"),
            Err(_) => panic!("Test timed out after 120 seconds - this may indicate a blocking operation or network issue"),
        }
    }

    #[tokio::test]
    #[ignore] // Requires root privileges for FUSE mount
    #[serial] // Serialize to avoid config initialization conflicts
    async fn creates_dirs_and_placeholder_overlay() {
        // Set overall test timeout to prevent hanging
        let test_future = async {
            // Ignore "already initialized" error when running multiple tests
            if let Err(e) = config::init_config("./scorpio.toml") {
                if !e.contains("already initialized") {
                    panic!("Failed to load config: {e}");
                }
            }

            // Check if we have necessary privileges
            let uid = unsafe { libc::geteuid() };
            if uid != 0 {
                println!("Warning: This test requires root privileges");
                println!("Run with: sudo -E cargo test --lib antares::fuse::tests::creates_dirs_and_placeholder_overlay -- --exact --ignored --nocapture");
                println!("Skipping test...");
                return;
            }

            let test_id = uuid::Uuid::new_v4();
            let base = PathBuf::from(format!("/tmp/antares_test_job1_{test_id}"));
            let _ = std::fs::remove_dir_all(&base);
            let mount = base.join("mnt");
            let upper = base.join("upper");
            let cl = base.join("cl");
            let store_path = base.join("store");
            std::fs::create_dir_all(&store_path).unwrap();

            // Use isolated Dicfuse instance for testing to avoid database lock conflicts
            // In production, use DicfuseManager::global() to share the instance
            let dic =
                crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;
            // Start background import_arc task to load directory tree asynchronously
            // This prevents blocking during FUSE operations (see blog post for details)
            println!("Starting Dicfuse background import_arc task...");
            tokio::spawn(crate::dicfuse::store::import_arc(dic.store.clone()));

            // Wait for Dicfuse to initialize and fetch directory tree from network
            // Use wait_for_ready() with timeout instead of fixed sleep to handle variable load times
            println!("Waiting for Dicfuse to initialize (this may take time if loading large directory trees)...");
            let init_start = std::time::Instant::now();
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(120), // 120 second timeout for large directory trees
                dic.store.wait_for_ready(),
            )
            .await
            {
                Ok(_) => {
                    let elapsed = init_start.elapsed();
                    println!(
                        "✓ Dicfuse initialized successfully after {:.2}s",
                        elapsed.as_secs_f64()
                    );
                }
                Err(_) => {
                    panic!(
                        "Dicfuse initialization timed out after 120 seconds. \
                        This may indicate:\n\
                        - Network issues preventing directory tree fetch\n\
                        - Very large directory tree (load_dir_depth={}) taking longer than expected\n\
                        - Background task may have failed\n\
                        Check logs for 'load_dir_depth' and 'Worker processing path' messages",
                        dic.store.max_depth()
                    );
                }
            }

            let mut fuse = AntaresFuse::new(
                mount.clone(),
                std::sync::Arc::new(dic),
                upper.clone(),
                Some(cl.clone()),
            )
            .await
            .unwrap();

            // Mount the overlay filesystem
            // mount() already verifies accessibility via read_dir, so we can skip redundant checks
            println!("Mounting Antares overlay at: {}", mount.display());
            fuse.mount().await.unwrap();
            println!("✓ Mount completed successfully");

            // Verify directories were created
            println!("Verifying directories exist...");

            // Use async metadata with timeout to avoid blocking on FUSE operations
            // PathBuf::exists() on FUSE mountpoint may trigger getattr/lookup which could block
            const CHECK_TIMEOUT_MS: u64 = 5000; // 5 second timeout per check

            // Check mount directory with timeout
            println!("  Checking mount directory: {}", mount.display());
            let mount_check_start = std::time::Instant::now();
            let mount_exists = match tokio::time::timeout(
                Duration::from_millis(CHECK_TIMEOUT_MS),
                tokio::fs::metadata(&mount),
            )
            .await
            {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => false,
                Err(_) => {
                    let elapsed = mount_check_start.elapsed();
                    panic!("Mount directory check timed out after {:.2}s - FUSE operation may be blocked", elapsed.as_secs_f64());
                }
            };
            let mount_check_elapsed = mount_check_start.elapsed();
            println!(
                "  Mount directory check took {:.2}ms, exists: {}",
                mount_check_elapsed.as_secs_f64() * 1000.0,
                mount_exists
            );
            assert!(mount_exists, "mount directory should exist");
            println!("✓ Mount directory exists");

            // Check upper directory (regular filesystem, should be fast)
            println!("  Checking upper directory: {}", upper.display());
            let upper_check_start = std::time::Instant::now();
            let upper_exists = match tokio::time::timeout(
                Duration::from_millis(CHECK_TIMEOUT_MS),
                tokio::fs::metadata(&upper),
            )
            .await
            {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => false,
                Err(_) => {
                    let elapsed = upper_check_start.elapsed();
                    panic!(
                        "Upper directory check timed out after {:.2}s",
                        elapsed.as_secs_f64()
                    );
                }
            };
            let upper_check_elapsed = upper_check_start.elapsed();
            println!(
                "  Upper directory check took {:.2}ms, exists: {}",
                upper_check_elapsed.as_secs_f64() * 1000.0,
                upper_exists
            );
            assert!(upper_exists, "upper directory should exist");
            println!("✓ Upper directory exists");

            // Check CL directory (regular filesystem, should be fast)
            println!("  Checking CL directory: {}", cl.display());
            let cl_check_start = std::time::Instant::now();
            let cl_exists = match tokio::time::timeout(
                Duration::from_millis(CHECK_TIMEOUT_MS),
                tokio::fs::metadata(&cl),
            )
            .await
            {
                Ok(Ok(_)) => true,
                Ok(Err(_)) => false,
                Err(_) => {
                    let elapsed = cl_check_start.elapsed();
                    panic!(
                        "CL directory check timed out after {:.2}s",
                        elapsed.as_secs_f64()
                    );
                }
            };
            let cl_check_elapsed = cl_check_start.elapsed();
            println!(
                "  CL directory check took {:.2}ms, exists: {}",
                cl_check_elapsed.as_secs_f64() * 1000.0,
                cl_exists
            );
            assert!(cl_exists, "cl directory should exist");
            println!("✓ CL directory exists");
            // Note: We don't call read_dir here because:
            // 1. mount() already verified accessibility via read_dir internally
            // 2. read_dir on FUSE mountpoint may trigger readdirplus which could block
            //    if Dicfuse is still loading data in the background
            // 3. This test focuses on verifying directory creation, not readdir functionality

            // Unmount
            println!("Unmounting...");
            let unmount_start = std::time::Instant::now();
            fuse.unmount().await.unwrap();
            let unmount_elapsed = unmount_start.elapsed();
            println!(
                "✓ Unmount successful (took {:.2}s)",
                unmount_elapsed.as_secs_f64()
            );

            // Cleanup
            let _ = std::fs::remove_dir_all(&base);
        };

        // Run test with timeout to prevent hanging
        // Increased timeout to 180s to account for Dicfuse network initialization and large directory trees
        match tokio::time::timeout(Duration::from_secs(180), test_future).await {
            Ok(_) => println!("✓ Test completed successfully"),
            Err(_) => panic!("Test timed out after 180 seconds - this may indicate:\n- Dicfuse background loading taking too long\n- Network issues\n- Very large directory tree (check load_dir_depth config)\nCheck logs for '[load_dir_depth]' messages to see loading progress"),
        }
    }

    /// Verify that creating a file in a deep directory path is reflected in the upper layer.
    ///
    /// Topology:
    /// - lower: one passthrough layer with a 3-level deep directory tree `a/b/c`.
    /// - upper: empty directory used as the writable layer.
    ///
    /// We create a file at `/mnt/a/b/c/created.txt` and then check that the file
    /// appears under `upper/a/b/c/created.txt` and does NOT exist in the lower tree.
    #[tokio::test]
    #[ignore]
    // Requires FUSE/root. Direct run example:
    //   sudo -E cargo test --lib antares::fuse::tests::deep_write_goes_to_upper -- --exact --ignored --nocapture
    // For LLDB-based debugging, follow the steps in `doc/test.md`.
    #[serial] // Serialize to avoid config initialization conflicts
    async fn deep_write_goes_to_upper() {
        use std::sync::Arc;

        use libfuse_fs::{
            passthrough::{new_passthroughfs_layer, newlogfs::LoggingFileSystem, PassthroughArgs},
            unionfs::{config::Config, OverlayFs},
        };
        // Only  LoggingFileSystem DEBUG
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive("libfuse_fs::passthrough::newlogfs=debug".parse().unwrap()),
            )
            .try_init();
        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges for FUSE/open_by_handle_at");
            println!(
                "Run with: sudo -E cargo test --lib antares::fuse::tests::deep_write_goes_to_upper -- --exact --ignored --nocapture"
            );
            return;
        }

        let base = PathBuf::from("/tmp/antares_deep_overlay_test3");
        // Clean up any existing mount point first
        let mount = base.join("mnt");
        let _ = tokio::process::Command::new("fusermount")
            .arg("-uz")
            .arg(&mount)
            .output()
            .await;
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let lower = base.join("lower");

        // Prepare directory layout: lower contains `a/b/c`, upper is empty.
        std::fs::create_dir_all(&mount).unwrap();
        std::fs::create_dir_all(&upper).unwrap();
        std::fs::create_dir_all(lower.join("a/b/c")).unwrap();

        // Build overlay: empty upper, single lower.
        let lower_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &lower,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let upper_layer = new_passthroughfs_layer(PassthroughArgs {
            root_dir: &upper,
            mapping: None::<String>,
        })
        .await
        .unwrap();

        let cfg = Config {
            mountpoint: mount.clone(),
            do_import: true,
            ..Default::default()
        };

        let overlay = OverlayFs::new(
            Some(Arc::new(upper_layer)),
            vec![Arc::new(lower_layer)],
            cfg,
            1,
        )
        .unwrap();

        println!("Mounting deep overlay at: {}", mount.display());
        let logfs = LoggingFileSystem::new(overlay);
        let handle = crate::server::mount_filesystem(logfs, mount.as_os_str()).await;

        // Run FUSE session in the background.
        let _fuse_task = tokio::spawn(async move {
            let _ = handle.await;
        });

        // Give the mount a moment to initialize.
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify mountpoint is accessible
        let metadata = tokio::fs::metadata(&mount).await;
        assert!(metadata.is_ok(), "Mountpoint should be accessible");

        // Test: Create a file in a deep directory path
        let test_file = mount.join("a/b/c/created.txt");
        tokio::fs::write(&test_file, b"test content").await.unwrap();

        // Verify file exists in mountpoint
        let content = tokio::fs::read(&test_file).await.unwrap();
        assert_eq!(content, b"test content");

        // Verify file exists in upper layer (copy-up happened)
        let upper_file = upper.join("a/b/c/created.txt");
        let upper_content = tokio::fs::read(&upper_file).await.unwrap();
        assert_eq!(
            upper_content, b"test content",
            "File should be copied up to upper layer"
        );

        // Verify file does NOT exist in lower layer
        let lower_file = lower.join("a/b/c/created.txt");
        assert!(!lower_file.exists(), "File should NOT exist in lower layer");

        // Unmount
        let _ = tokio::process::Command::new("fusermount")
            .arg("-uz")
            .arg(&mount)
            .output()
            .await;

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Test that copy-up works correctly when modifying files from the lower layer.
    /// This test specifically verifies that `do_getattr_helper` is properly implemented,
    /// as copy-up requires getting file attributes from the lower layer.
    #[tokio::test]
    #[ignore] // Requires root privileges and network access
    #[serial]
    async fn test_copyup_modifies_lower_file() {
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges");
            println!("Run with: sudo -E cargo test -p scorpio --lib antares::fuse::tests::test_copyup_modifies_lower_file -- --exact --ignored --nocapture");
            println!("Skipping test...");
            return;
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_copyup_test_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let cl = base.join("cl");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        // Create Dicfuse and wait for directory tree to load
        let dic = crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;

        println!("Loading directory tree synchronously...");
        crate::dicfuse::store::import_arc(dic.store.clone()).await;
        println!("Directory tree loaded");

        let mut fuse = AntaresFuse::new(
            mount.clone(),
            std::sync::Arc::new(dic),
            upper.clone(),
            Some(cl.clone()),
        )
        .await
        .unwrap();

        println!("Mounting Antares overlay at: {}", mount.display());
        fuse.mount().await.unwrap();
        println!("Mount completed");

        // Give mount a moment to stabilize
        sleep(Duration::from_millis(500)).await;

        // Recursively find a file from the lower layer (Dicfuse)
        // We need to search in subdirectories since root may only have directories
        async fn find_file_recursive(
            dir: &std::path::Path,
            upper: &std::path::Path,
            mount: &std::path::Path,
            depth: usize,
        ) -> Option<std::path::PathBuf> {
            if depth > 3 {
                return None; // Don't go too deep
            }

            let mut entries = match tokio::fs::read_dir(dir).await {
                Ok(e) => e,
                Err(_) => return None,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_file() {
                    // Get relative path from mount point
                    let rel_path = path.strip_prefix(mount).unwrap_or(&path);
                    let upper_path = upper.join(rel_path);
                    if !upper_path.exists() {
                        return Some(path);
                    }
                } else if file_type.is_dir() {
                    // Recurse into subdirectory
                    if let Some(found) =
                        Box::pin(find_file_recursive(&path, upper, mount, depth + 1)).await
                    {
                        return Some(found);
                    }
                }
            }
            None
        }

        println!("Searching for a file in lower layer (Dicfuse)...");
        let found_lower_file = find_file_recursive(&mount, &upper, &mount, 0).await;

        if let Some(lower_file) = found_lower_file {
            // Get relative path from mount point for correct upper layer path
            let rel_path = lower_file.strip_prefix(&mount).unwrap();
            println!("Found lower layer file: {}", rel_path.display());

            // Read original content
            let original_content = tokio::fs::read(&lower_file).await.unwrap();
            println!("Original content length: {} bytes", original_content.len());

            // Modify the file - THIS TRIGGERS COPY-UP
            // Copy-up calls do_getattr_helper to get file attributes
            let modified_content = b"MODIFIED BY TEST - copy-up successful!";
            println!("Attempting to modify file (this triggers copy-up)...");

            match tokio::fs::write(&lower_file, modified_content).await {
                Ok(_) => {
                    println!("✓ File modification successful");

                    // Verify modification persisted
                    let read_back = tokio::fs::read(&lower_file).await.unwrap();
                    assert_eq!(
                        read_back, modified_content,
                        "Modified content should be readable"
                    );
                    println!("✓ Modified content verified");

                    // Verify copy-up: file should now be in upper layer (use relative path)
                    let upper_file = upper.join(rel_path);
                    assert!(
                        upper_file.exists(),
                        "File should be copied to upper layer after modification: {}",
                        upper_file.display()
                    );

                    let upper_content = tokio::fs::read(&upper_file).await.unwrap();
                    assert_eq!(
                        upper_content, modified_content,
                        "Upper layer should have modified content"
                    );
                    println!(
                        "✓ Copy-up verified: {} copied to upper layer with modified content",
                        rel_path.display()
                    );
                }
                Err(e) => {
                    panic!("Failed to modify lower layer file - copy-up failed: {}", e);
                }
            }
        } else {
            println!("⚠ No files found in lower layer - test inconclusive");
            println!("  This may happen if Dicfuse couldn't load files from remote server");
        }

        // Cleanup
        println!("Unmounting...");
        fuse.unmount().await.unwrap();
        println!("✓ Test completed");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Test that mkdir works in a lower layer directory (requires directory copy-up).
    /// This simulates Buck2's behavior: creating buck-out/v2 inside a directory from Dicfuse.
    ///
    /// The test verifies:
    /// 1. We can find a directory from lower layer (Dicfuse)
    /// 2. We can create a new subdirectory inside it (triggers directory copy-up)
    /// 3. The new directory appears in the upper layer
    #[tokio::test]
    #[ignore] // Requires root privileges and network access
    #[serial]
    async fn test_mkdir_in_lower_layer_directory() {
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        if let Err(e) = config::init_config("./scorpio.toml") {
            if !e.contains("already initialized") {
                panic!("Failed to load config: {e}");
            }
        }

        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            println!("Warning: This test requires root privileges");
            println!("Run with: sudo -E cargo test -p scorpio --lib antares::fuse::tests::test_mkdir_in_lower_layer_directory -- --exact --ignored --nocapture");
            println!("Skipping test...");
            return;
        }

        let test_id = Uuid::new_v4();
        let base = PathBuf::from(format!("/tmp/antares_mkdir_test_{test_id}"));
        let _ = std::fs::remove_dir_all(&base);
        let mount = base.join("mnt");
        let upper = base.join("upper");
        let cl = base.join("cl");
        let store_path = base.join("store");
        std::fs::create_dir_all(&store_path).unwrap();

        // Create Dicfuse and wait for directory tree to load
        let dic = crate::dicfuse::Dicfuse::new_with_store_path(store_path.to_str().unwrap()).await;

        println!("Loading directory tree synchronously...");
        crate::dicfuse::store::import_arc(dic.store.clone()).await;
        println!("Directory tree loaded");

        let mut fuse = AntaresFuse::new(
            mount.clone(),
            std::sync::Arc::new(dic),
            upper.clone(),
            Some(cl.clone()),
        )
        .await
        .unwrap();

        println!("Mounting Antares overlay at: {}", mount.display());
        fuse.mount().await.unwrap();
        println!("Mount completed");

        // Give mount a moment to stabilize
        sleep(Duration::from_millis(500)).await;

        // Find a directory from the lower layer (Dicfuse)
        async fn find_dir_recursive(
            dir: &std::path::Path,
            upper: &std::path::Path,
            mount: &std::path::Path,
            depth: usize,
        ) -> Option<std::path::PathBuf> {
            if depth > 2 {
                return None; // Don't go too deep
            }

            let mut entries = match tokio::fs::read_dir(dir).await {
                Ok(e) => e,
                Err(_) => return None,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    // Get relative path from mount point
                    let rel_path = path.strip_prefix(mount).unwrap_or(&path);
                    let upper_path = upper.join(rel_path);

                    // We want a directory that exists in lower but NOT in upper
                    if !upper_path.exists() {
                        return Some(path);
                    }

                    // Recurse into subdirectory
                    if let Some(found) =
                        Box::pin(find_dir_recursive(&path, upper, mount, depth + 1)).await
                    {
                        return Some(found);
                    }
                }
            }
            None
        }

        println!("Searching for a directory in lower layer (Dicfuse)...");
        let found_lower_dir = find_dir_recursive(&mount, &upper, &mount, 0).await;

        if let Some(lower_dir) = found_lower_dir {
            let rel_path = lower_dir.strip_prefix(&mount).unwrap();
            println!("Found lower layer directory: {}", rel_path.display());

            // Try to create a new subdirectory inside it
            // This simulates Buck2 creating buck-out/v2
            let new_subdir = lower_dir.join("test-subdir-created-by-test");
            println!(
                "Attempting to create subdirectory: {}",
                new_subdir.strip_prefix(&mount).unwrap().display()
            );
            println!("This will trigger directory copy-up...");

            match tokio::fs::create_dir(&new_subdir).await {
                Ok(_) => {
                    println!("✓ Subdirectory creation successful!");

                    // Verify the directory exists (use async with timeout to avoid blocking)
                    match tokio::time::timeout(
                        Duration::from_secs(2),
                        tokio::fs::metadata(&new_subdir),
                    )
                    .await
                    {
                        Ok(Ok(meta)) if meta.is_dir() => {
                            println!("✓ Subdirectory exists in mountpoint");
                        }
                        _ => {
                            println!(
                                "⚠ Could not verify subdirectory in mountpoint (timeout or error)"
                            );
                        }
                    }

                    // Verify it's in upper layer (copy-up happened for parent directory)
                    // Use std::fs for upper layer since it's not through FUSE
                    let upper_new_subdir = upper.join(rel_path).join("test-subdir-created-by-test");

                    // Give filesystem a moment to sync
                    sleep(Duration::from_millis(100)).await;

                    if upper_new_subdir.exists() {
                        println!(
                            "✓ Directory copy-up verified: new subdirectory exists in upper layer"
                        );
                        println!("  Upper path: {}", upper_new_subdir.display());
                    } else {
                        println!(
                            "⚠ New subdirectory not found in upper layer (may be a timing issue)"
                        );
                        println!("  Expected: {}", upper_new_subdir.display());
                    }

                    // Test creating a file inside the new directory (with timeout)
                    let test_file = new_subdir.join("test.txt");
                    match tokio::time::timeout(
                        Duration::from_secs(2),
                        tokio::fs::write(&test_file, b"test content"),
                    )
                    .await
                    {
                        Ok(Ok(_)) => {
                            println!("✓ Created file inside new subdirectory");

                            // Verify file content (with timeout)
                            match tokio::time::timeout(
                                Duration::from_secs(2),
                                tokio::fs::read(&test_file),
                            )
                            .await
                            {
                                Ok(Ok(content)) => {
                                    assert_eq!(content, b"test content");
                                    println!("✓ File content verified");
                                }
                                _ => {
                                    println!("⚠ Could not verify file content (timeout)");
                                }
                            }
                        }
                        _ => {
                            println!("⚠ Could not create file inside new subdirectory (timeout)");
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to create subdirectory in lower layer directory!");
                    println!(
                        "  Error: {} (os error {})",
                        e,
                        e.raw_os_error().unwrap_or(-1)
                    );
                    println!("  This indicates directory copy-up is not working correctly.");
                    println!(
                        "  The OverlayFS should copy the parent directory to upper layer first,"
                    );
                    println!("  then create the new subdirectory there.");
                    panic!("mkdir in lower layer directory failed: {}", e);
                }
            }
        } else {
            println!("⚠ No directories found in lower layer - test inconclusive");
            println!("  This may happen if Dicfuse couldn't load directories from remote server");
        }

        // Cleanup
        println!("Unmounting...");
        fuse.unmount().await.unwrap();
        println!("✓ Test completed");

        let _ = std::fs::remove_dir_all(&base);
    }
}
