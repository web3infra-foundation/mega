use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::prelude::*,
    os::{fd::IntoRawFd, unix::prelude::FileExt},
    sync::RwLock,
    time::SystemTime,
};

use crate::inode::{InodeAttributes, InodeKind};

const FILE_PREFIX: &str = "tmp_";

pub struct TemporaryFileManager {
    caches: HashMap<u64, TmpFile>,
    tmp_dir_prefix: String,
    ops_seq: Vec<Ops>,
    counter: u64,
}

enum Ops {
    Create(InodeAttributes),
    Delete(InodeKind, String),
    Alter(InodeAttributes),
    Update(u64, String),
}

impl TemporaryFileManager {
    pub fn new(data_dir: String) -> Self {
        std::fs::create_dir(data_dir.clone()).unwrap();
        Self {
            caches: HashMap::new(),
            tmp_dir_prefix: data_dir,
            ops_seq: Vec::new(),
            counter: 1,
        }
    }

    pub fn new_file(&mut self, ino: u64, attr: InodeAttributes) {
        let file_name = FILE_PREFIX.to_string() + &self.counter.to_string();
        self.counter += 1;
        let path = self.tmp_dir_prefix.clone() + "/" + &file_name;
        let tmp = TmpFile {
            path: path.clone(),
            lock: RwLock::new(()),
        };

        std::fs::File::create(path).unwrap();
        self.caches.insert(ino, tmp);
        self.ops_create(attr);
    }

    pub fn tmp_file(&mut self, ino: u64, data: &[u8]) {
        let file_name = FILE_PREFIX.to_string() + &self.counter.to_string();
        self.counter += 1;
        let path = self.tmp_dir_prefix.clone() + "/" + &file_name;
        let mut tmp = TmpFile {
            path,
            lock: RwLock::new(()),
        };
        tmp.write(data);
        self.caches.insert(ino, tmp);
    }

    pub fn new_dir(&mut self, ino: u64, attr: InodeAttributes) {
        let tmp = TmpFile {
            path: "".to_owned(),
            lock: RwLock::new(()),
        };
        self.caches.insert(ino, tmp);
        self.ops_create(attr);
    }

    pub fn rm_file(&mut self, ino: u64, id: String) {
        let tmp = self.caches.remove(&ino).unwrap();
        std::fs::remove_file(tmp.path).unwrap();
        self.ops_delete(InodeKind::File, id);
    }

    pub fn rm_dir(&mut self, ino: u64, path: String) {
        self.caches.remove(&ino).unwrap();
        self.ops_delete(InodeKind::Directory, path);
    }

    pub fn append_content(&mut self, ino: u64, data: &[u8], id: String) {
        let tmp = self.caches.get_mut(&ino).unwrap();
        tmp.write(data);
        self.ops_update(id, ino);
    }

    pub fn read(&self, ino: u64, buf: &mut [u8], offset: u64) {
        let tmp = self.caches.get(&ino).unwrap();
        tmp.read_exact(buf, offset);
    }

    pub fn fallocate(&mut self, ino: u64, mode: i32, offset: i64, len: i64, id: String) {
        let tmp = self.caches.get(&ino).unwrap();
        {
            let _guard = tmp.lock.write().unwrap();
            let file = OpenOptions::new()
                .write(true)
                .open(tmp.path.clone())
                .unwrap();
            unsafe {
                libc::fallocate64(file.into_raw_fd(), mode, offset, len);
            }
        }
        self.ops_update(id, ino);
    }

    pub fn ops_create(&mut self, attr: InodeAttributes) {
        self.ops_seq.push(Ops::Create(attr));
    }

    pub fn ops_delete(&mut self, kind: InodeKind, target: String) {
        self.ops_seq.push(Ops::Delete(kind, target));
    }

    pub fn ops_alter(&mut self, attr: InodeAttributes) {
        self.ops_seq.push(Ops::Alter(attr));
    }

    pub fn ops_update(&mut self, id: String, ino: u64) {
        self.ops_seq.push(Ops::Update(ino, id));
    }

    pub fn exist(&self, ino: u64) -> bool {
        self.caches.contains_key(&ino)
    }

    pub fn generate_seq(&mut self) -> String {
        let seq: Vec<Ops> = self.ops_seq.drain(..).collect();

        let res:Vec<String> = seq.into_iter().map(|ops|match ops {
                    Ops::Alter(attr) => {
                        serde_json::json!({
                            "operation": "alter",
                            "attr": {
                                "id": attr.id,
                                "size": attr.size,
                                "name": attr.name,
                                "kind": attr.kind,
                                "path": attr.path,
                                "mtime": attr.mtime.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(),
                                "ctime": attr.ctime.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(),
                                "permissions": attr.permissions,
                            }
                        }).to_string()
                    }
                    Ops::Create(attr) => {
                        serde_json::json!({
                            "operation": "create",
                            "attr": {
                                "id": attr.id,
                                "size": attr.size,
                                "name": attr.name,
                                "kind": attr.kind,
                                "path": attr.path,
                                "mtime": attr.mtime.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(),
                                "ctime": attr.ctime.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos(),
                                "permissions": attr.permissions,
                            }
                        }).to_string()
                    }
                    Ops::Delete(kind, target) => {
                        serde_json::json!({
                            "operation": "delete",
                            "kind":kind,
                            "target": target
                        }).to_string()
                    }
                    Ops::Update(ino, id) => {
                        let tmp=self.caches.get(&ino).unwrap();
                        let data=tmp.read_all();
                        serde_json::json!({
                            "operation": "update",
                            "id":id,
                            "data":data
                        }).to_string()
                    }
                }).collect();
        let size = res.len();
        let mut json = "[".to_owned();
        for (index, item) in res.into_iter().enumerate() {
            if index + 1 == size {
                json = json + &item + "\n"
            } else {
                json = json + &item + ",\n"
            }
        }
        json + "]"
    }

    pub fn clean_temp(&mut self) {
        std::fs::remove_dir_all(self.tmp_dir_prefix.clone()).unwrap();
    }
}

pub struct TmpFile {
    path: String,
    lock: RwLock<()>,
}

impl TmpFile {
    fn write(&mut self, data: &[u8]) {
        let _guard = self.lock.write().unwrap();
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(self.path.clone())
            .unwrap();
        file.write_all(data).unwrap();
    }

    fn read_all(&self) -> Vec<u8> {
        let _guard = self.lock.read().unwrap();
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .open(self.path.clone())
            .unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        buf
    }

    fn read_exact(&self, buf: &mut [u8], offset: u64) {
        let _guard = self.lock.read().unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(self.path.clone())
            .unwrap();
        file.read_exact_at(buf, offset).unwrap();
    }
}
