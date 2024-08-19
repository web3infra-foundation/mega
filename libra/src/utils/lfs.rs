use std::fs::File;
use std::{fs, io};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use lazy_static::lazy_static;
use path_abs::{PathInfo, PathOps};
use regex::Regex;
use sha2::{Digest, Sha256};
use wax::Pattern;
use crate::utils::{path, util};
use crate::utils::path_ext::PathExt;

lazy_static! {
    static ref LFS_PATTERNS: Vec<String> = { // cache
        let attr_path = path::attributes().to_string_or_panic();
        extract_lfs_patterns(&attr_path).unwrap()
    };
}

/// Check if a file is LFS tracked
/// - support Glob pattern matching (TODO: support .gitignore patterns)
/// - only check root attributes file now, should check all attributes files in sub-dirs
/// - absolute path or relative path to workdir
pub fn is_lfs_tracked<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    if LFS_PATTERNS.is_empty() {
        return false;
    }

    let path = util::to_workdir_path(path);
    let glob = wax::any(LFS_PATTERNS.iter().map(|s| s.as_str()).collect::<Vec<_>>()).unwrap();
    glob.is_match(path.to_str().unwrap())
}

const LFS_VERSION: &str = "https://git-lfs.github.com/spec/v1";
const LFS_HASH_ALGO: &str = "sha256";

/// Generate lfs pointer file
/// - return (pointer content, file hash)
/// - absolute path
pub fn generate_pointer_file<P>(path: P) -> (String, String)
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    // calc file hash without type
    let file_hash = calc_lfs_file_hash(path).unwrap();

    let pointer = format!("version {}\noid {}:{}\nsize {}\n",
            LFS_VERSION, LFS_HASH_ALGO, file_hash, path.metadata().unwrap().len());
    (pointer, file_hash)
}

/// Copy LFS file to `.libra/lfs/objects`
/// - absolute path
pub fn backup_lfs_file<P>(path: P, hash: &str) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let backup_path = util::storage_path()
        .join("lfs/objects")
        .join(hash[..2].to_string())
        .join(hash[2..4].to_string())
        .join(hash);
    fs::create_dir_all(backup_path.parent().unwrap())?;
    fs::copy(path, backup_path)?;
    Ok(())
}

/// SHA256 without type
/// TODO: performance optimization, 200MB 4s now, slower than `sha256sum`
pub fn calc_lfs_file_hash<P>(path: P) -> io::Result<String>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let mut hash = Sha256::new();
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let file_hash = hex::encode(hash.finalize());
    Ok(file_hash)
}

/// Extract LFS patterns from `.libra_attributes` file
pub fn extract_lfs_patterns(file_path: &str) -> io::Result<Vec<String>> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // ' ' needs '\' before it to be escaped
    let re = Regex::new(r"^\s*(([^\s#\\]|\\ )+)").unwrap();

    let mut patterns = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if !line.contains("filter=lfs") {
            continue;
        }
        if let Some(cap) = re.captures(&line) {
            if let Some(pattern) = cap.get(1) {
                let pattern = pattern.as_str().replace(r"\ ", " ");
                patterns.push(pattern);
            }
        }
    }

    Ok(patterns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pointer_file() {
        let path = Path::new("../tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");
        let (pointer, _oid) = generate_pointer_file(path);
        print!("{}", pointer);
    }
}