use std::{
    io::{BufReader, Read},
    path::Path,
};

use crate::utils::lfs::generate_pointer_file;
use mercury::{hash::SHA1, internal::object::blob::Blob};

use crate::scolfs::lfs::backup_lfs_file;
#[allow(unused)]
pub trait BlobExt {
    fn load(hash: &SHA1) -> Blob;
    fn from_file(path: impl AsRef<Path>) -> Blob;
    fn from_lfs_file(path: impl AsRef<Path>) -> Blob;
    fn save(&self) -> SHA1;
}
impl BlobExt for Blob {
    fn load(_hash: &SHA1) -> Blob {
        todo!()
    }

    /// Create a blob from a file
    /// - `path`: absolute  or relative path to current dir
    fn from_file(path: impl AsRef<Path>) -> Blob {
        let mut data = Vec::new();
        let file = std::fs::File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut data).unwrap();
        Blob::from_content_bytes(data)
    }

    /// Create a blob from an LFS file
    /// - include: create a pointer file & copy the file to `.libra/lfs/objects`
    /// - `path`: absolute  or relative path to current dir
    fn from_lfs_file(path: impl AsRef<Path>) -> Blob {
        let (pointer, oid) = generate_pointer_file(&path);
        tracing::debug!("\n{}", pointer);
        backup_lfs_file(&path, &oid).unwrap();
        Blob::from_content(&pointer)
    }

    fn save(&self) -> SHA1 {
        todo!();
    }
}
