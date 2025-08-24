use flate2::read::ZlibDecoder;
use mercury::errors::GitError;
use mercury::hash::SHA1;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
/// Helper function to read and decompress a git object from the object database.
pub fn read_git_object(git_dir: &Path, hash: &SHA1) -> Result<Vec<u8>, GitError> {
    let hash_str = hash.to_string();
    let object_path = git_dir
        .join("objects")
        .join(&hash_str[..2])
        .join(&hash_str[2..]);

    let file = fs::File::open(object_path)?;
    let mut decoder = ZlibDecoder::new(file);
    let mut buffer = Vec::new();
    decoder.read_to_end(&mut buffer)?;

    // The buffer now contains "<type> <size>\0<content>", where <type> is the git object type (e.g., commit, tree, blob, tag)
    // Strip the header (which contains the object type and size) to obtain only the object content.
    if let Some(header_end) = buffer.iter().position(|&b| b == 0) {
        Ok(buffer[header_end + 1..].to_vec())
    } else {
        Err(GitError::InvalidObjectInfo(
            "Could not find object header terminator".to_string(),
        ))
    }
}

/// Helper function to write a git object to the object database.
pub fn write_git_object(git_dir: &Path, object_type: &str, data: &[u8]) -> Result<SHA1, GitError> {
    let header = format!("{} {}\0", object_type, data.len());
    let mut content = header.into_bytes();
    content.extend_from_slice(data);
    let hash = SHA1::new(&content);
    let hash_str = hash.to_string();

    let object_path = git_dir
        .join("objects")
        .join(&hash_str[..2])
        .join(&hash_str[2..]);

    if !object_path.exists() {
        fs::create_dir_all(object_path.parent().unwrap())?;
        let file = fs::File::create(object_path)?;
        let mut encoder = flate2::write::ZlibEncoder::new(file, flate2::Compression::default());
        encoder.write_all(&content)?;
        encoder.finish()?;
    }

    Ok(hash)
}
