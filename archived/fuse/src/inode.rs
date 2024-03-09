use fuser::{FileAttr, FUSE_ROOT_ID};

use crate::req_remote::InodeContent;

use super::common::*;
use std::{
    fmt::Display,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

use fuser::FileType;
use serde::{Deserialize, Serialize};

static INO_ALLOCATOR: AtomicU64 = AtomicU64::new(FUSE_ROOT_ID + 1);

fn alloc_ino() -> u64 {
    INO_ALLOCATOR.fetch_add(1, Ordering::SeqCst)
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InodeKind {
    File,
    Directory,
}

#[derive(Clone)]
pub struct Inode {
    pub ino: u64,
    pub parent_ino: u64,
    pub children_ino: Vec<u64>,
    pub attr: InodeAttributes,
}

#[derive(Clone,Serialize, Deserialize)]
pub struct InodeAttributes {
    pub id: String,
    pub size: u64,
    pub name: String,
    pub kind: InodeKind,
    pub path: String,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
    pub permissions: u16,
}

impl Inode {
    pub fn new(parent_ino: u64, attr: InodeAttributes) -> Self {
        Self {
            ino: alloc_ino(),
            parent_ino,
            children_ino: Vec::new(),
            attr,
        }
    }

    pub fn insert_child(&mut self, child: u64) {
        self.children_ino.push(child);
    }
    pub fn remove_child(&mut self, child: u64) {
        let mut index = 0;
        for ele in self.children_ino.iter() {
            if *ele == child {
                break;
            }
            index += 1;
        }
        self.children_ino.remove(index);
    }

    pub fn file_attr(&self) -> FileAttr {
        let attrs = &self.attr;
        FileAttr {
            ino: self.ino,
            size: attrs.size,
            blocks: attrs.size / (BLOCK_SIZE as u64) + 1,
            atime: attrs.mtime,
            mtime: attrs.mtime,
            ctime: attrs.ctime,
            crtime: attrs.ctime,
            kind: attrs.kind.into(),
            perm: attrs.permissions,
            nlink: DEFAULT_HARD_LINKS,
            uid: uid(),
            gid: gid(),
            rdev: RDEV,
            blksize: BLOCK_SIZE,
            flags: FLAGS,
        }
    }
}

impl InodeAttributes {
    pub fn new(name:String,kind:InodeKind,path:String) -> Self {
        let now=SystemTime::now();
        Self {
            id: alloc_ino().to_string(),
            size: BLOCK_SIZE as u64,
            name,
            kind,
            path,
            mtime: now,
            ctime: now,
            permissions: DEFAULT_PERMISSIONS,
        }
    }
}

impl From<InodeContent> for InodeAttributes {
    fn from(value: InodeContent) -> Self {
        let mtime = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(value.mtime))
            .unwrap();
        let ctime = SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(value.ctime))
            .unwrap();
        Self {
            kind: value.kind.into(),
            id: value.id,
            size: value.size,
            name: value.name,
            path: value.path,
            mtime,
            ctime,
            permissions: value.permissions,
        }
    }
}

impl From<String> for InodeKind {
    fn from(value: String) -> Self {
        if value.eq("file") {
            Self::File
        } else {
            Self::Directory
        }
    }
}

impl Display for InodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InodeKind::Directory => write!(f, "file"),
            InodeKind::File => write!(f, "directory"),
        }
    }
}

impl From<InodeKind> for FileType {
    fn from(value: InodeKind) -> Self {
        match value {
            InodeKind::Directory => FileType::Directory,
            InodeKind::File => FileType::RegularFile,
        }
    }
}

pub fn root_node(fs_name: &str) -> Inode {
    let attr = InodeAttributes {
        id: fs_name.to_string(),
        size: BLOCK_SIZE as u64,
        name: fs_name.to_string(),
        path: "".to_owned(),
        kind: InodeKind::Directory,
        mtime: SystemTime::now(),
        ctime: SystemTime::now(),
        permissions: DEFAULT_PERMISSIONS,
    };
    Inode {
        ino: FUSE_ROOT_ID,
        parent_ino: FUSE_ROOT_ID,
        children_ino: Vec::new(),
        attr,
    }
}
