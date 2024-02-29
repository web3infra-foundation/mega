use std::io::Cursor;

use venus::internal::object::blob::Blob;
use venus::internal::object::types::ObjectType;
use venus::internal::object::{utils, ObjectTrait};
use venus::internal::zlib::stream::inflate::ReadBoxed;

pub fn generate_git_keep() -> Blob {
    let git_keep_content = String::from("This file was used to maintain the git tree");
    let blob_content = Cursor::new(utils::compress_zlib(git_keep_content.as_bytes()).unwrap());
    let mut buf = ReadBoxed::new(blob_content, ObjectType::Blob, git_keep_content.len());
    Blob::from_buf_read(&mut buf, git_keep_content.len())
}
