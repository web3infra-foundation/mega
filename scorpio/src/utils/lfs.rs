use ignore::{gitignore::GitignoreBuilder, Match};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use ring::digest::{Context, SHA256};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::{fs, io};
use url::Url;
use wax::Pattern;

// TODO: Replace with proper working directory resolution for scorpio
fn working_dir() -> PathBuf {
    std::env::current_dir().unwrap()
}

// TODO: Replace with proper attributes file path for scorpio
fn attributes_path() -> PathBuf {
    working_dir().join(".libra_attributes")
}

// TODO: Replace with proper storage path resolution for scorpio
pub fn storage_path() -> PathBuf {
    working_dir().join(".libra")
}

static LFS_PATTERNS: Lazy<Vec<String>> = Lazy::new(|| {
    let attr_path = attributes_path();
    if attr_path.exists() {
        extract_lfs_patterns(attr_path.to_str().unwrap()).unwrap_or_default()
    } else {
        Vec::new()
    }
});

pub static LFS_HEADERS: Lazy<HeaderMap> = Lazy::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.git-lfs+json"),
    );
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.git-lfs+json"),
    );
    headers
});

/// Check if a file is LFS tracked
/// - support Glob pattern matching
/// - only check root attributes file now, should check all attributes files in sub-dirs
/// - absolute path
pub fn is_lfs_tracked<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    if LFS_PATTERNS.is_empty() {
        return false;
    }

    let patterns = LFS_PATTERNS.iter().map(|s| s.as_str()).collect::<Vec<_>>();

    let mut gitignore = GitignoreBuilder::new(working_dir());
    patterns.iter().for_each(|&s| {
        let _ = gitignore.add_line(None, s);
    });
    let gitignore = gitignore.build().unwrap();
    let match_gitignore = gitignore.matched(&path, false);
    let gitignore_matched = matches!(match_gitignore, Match::Ignore(_));

    // Convert to relative path from working directory
    let path = path.as_ref();
    let working_dir = working_dir();
    let relative_path = if path.is_absolute() && path.starts_with(&working_dir) {
        path.strip_prefix(&working_dir).unwrap_or(path)
    } else {
        path
    };

    let glob = wax::any(patterns).unwrap();
    glob.is_match(relative_path.to_str().unwrap()) || gitignore_matched
}

const LFS_VERSION: &str = "https://git-lfs.github.com/spec/v1";
/// This is the original & default transfer adapter. All Git LFS clients and servers SHOULD support it.
pub const LFS_TRANSFER_API: &str = "basic";
pub const LFS_HASH_ALGO: &str = "sha256";
const LFS_OID_LEN: usize = 64;
const LFS_POINTER_MAX_SIZE: usize = 300; // bytes

/// Generate lfs pointer file string
/// - return (pointer content, lfs oid)
/// - absolute path
pub fn generate_pointer_file(path: impl AsRef<Path>) -> (String, String) {
    let path = path.as_ref();
    // calc file hash without type
    let oid = calc_lfs_file_hash(path).unwrap();

    let pointer = format_pointer_string(&oid, path.metadata().unwrap().len());
    (pointer, oid)
}

pub fn format_pointer_string(oid: &str, size: u64) -> String {
    format!("version {LFS_VERSION}\noid {LFS_HASH_ALGO}:{oid}\nsize {size}\n")
}

/// Generate LFS Server Url from repo Url.
/// By default, Git LFS will append `.git/info/lfs` to the end of a Git remote url to build the LFS server URL.
/// [doc: server-discovery](https://github.com/git-lfs/git-lfs/blob/main/docs/api/server-discovery.md)
/// - like `https://git-server.com/foo/bar.git/info/lfs`
/// - support ssh & https & git@ format
fn generate_git_lfs_server_url(mut url: String) -> String {
    if url.ends_with('/') {
        url.pop();
    }
    if !url.ends_with(".git") {
        url.push_str(".git");
    }
    url.push_str("/info/lfs");

    if url.starts_with("git@") {
        // git@git-server.com:foo/bar.git
        url = "https://".to_string() + &url[4..].replace(":", "/");
    } else if url.starts_with("ssh://") {
        // ssh://git-server.com/foo/bar.git
        url = "https://".to_string() + &url[6..];
    }

    url
}

/// Generate Mono LFS Server Url from repo Url.
/// - Just get domain with port
/// ### Example
/// https://github.com/git-lfs/git-lfs/blob/main/docs/api/locking.md -> https://github.com
///
/// http://localhost:8000/xxx/yyy -> http://localhost:8000
fn generate_mono_lfs_server_url(url: String) -> String {
    let url = Url::parse(&url).unwrap();
    match url.port() {
        None => {
            format!("{}://{}", url.scheme(), url.host().unwrap())
        }
        Some(port) => {
            format!("{}://{}:{}", url.scheme(), url.host().unwrap(), port)
        }
    }
}

/// Generate LFS Server Url from repo Url.
/// - Automatically detect git or mono repo by domain
/// - Caution: without trailing slash `/`
pub fn generate_lfs_server_url(url_str: String) -> String {
    let url = Url::parse(&url_str);
    if url.is_err() {
        // maybe start with `git@`
        return generate_git_lfs_server_url(url_str);
    }
    let url = url.unwrap();
    match url.domain() {
        Some(domain) => {
            if domain == "github.com" || domain == "gitee.com" {
                generate_git_lfs_server_url(url_str)
            } else {
                generate_mono_lfs_server_url(url_str)
            }
        }
        None => {
            // IP address, like http://127.0.0.1:8000
            generate_mono_lfs_server_url(url_str)
        }
    }
}

/// Generate LFS cache path, in `.libra/lfs/objects`
pub fn lfs_object_path(oid: &str) -> PathBuf {
    storage_path()
        .join("lfs/objects")
        .join(&oid[..2])
        .join(&oid[2..4])
        .join(oid)
}

/// Copy LFS file to `.libra/lfs/objects`
/// - absolute path
pub fn backup_lfs_file<P>(path: P, oid: &str) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let backup_path = lfs_object_path(oid);
    if !backup_path.exists() {
        fs::create_dir_all(backup_path.parent().unwrap())?;
        fs::copy(path, backup_path)?;
    }
    Ok(())
}

/// SHA256 without type
// `ring` crate is much faster than `sha2` crate ( > 10 times)
pub fn calc_lfs_file_hash<P>(path: P) -> io::Result<String>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let mut hash = Context::new(&SHA256);
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
    let file_hash = hex::encode(hash.finish().as_ref());
    Ok(file_hash)
}

/// Check if `data` is an LFS pointer, return `oid` & `size`
pub fn parse_pointer_data(data: &[u8]) -> Option<(String, u64)> {
    if data.len() > LFS_POINTER_MAX_SIZE {
        return None;
    }
    // Start with format `version ...`
    let prefix = format!("version {LFS_VERSION}\noid {LFS_HASH_ALGO}:");
    if let Some(data) = data.strip_prefix(prefix.as_bytes()) {
        if data.len() > LFS_OID_LEN && data[LFS_OID_LEN] == b'\n' {
            // check `oid` length
            let oid = String::from_utf8(data[..LFS_OID_LEN].to_vec()).unwrap();
            let size_prefix = format!("{oid}\nsize ");
            if let Some(data) = data.strip_prefix(size_prefix.as_bytes()) {
                let data = String::from_utf8(data[..].to_vec()).unwrap();
                if let Ok(size) = data.trim_end().parse::<u64>() {
                    return Some((oid, size));
                }
            }
        }
    }
    None
}

/// Read max LFS_POINTER_MAX_SIZE bytes
pub fn parse_pointer_file(path: impl AsRef<Path>) -> io::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut buffer = [0; LFS_POINTER_MAX_SIZE];
    let bytes_read = file.read(&mut buffer)?;
    if let Some((oid, size)) = parse_pointer_data(&buffer[..bytes_read]) {
        return Ok((oid, size));
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid LFS pointer file",
    ))
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
    fn test_is_pointer_file() {
        let data =
            b"version https://git-lfs.github.com/spec/v1\noid sha256:3b2c9e5f8e6a8b7a9c8d6e5f7a9b8c7d6e5f8a9b7a9c8d6e5f8a9b7a9c8d6e51\nsize 1234\n";
        assert!(parse_pointer_data(data).is_some());
    }

    #[test]
    fn test_gen_git_lfs_server_url() {
        const LFS_SERVER_URL: &str = "https://github.com/web3infra-foundation/mega.git/info/lfs";
        let url = "https://github.com/web3infra-foundation/mega".to_owned();
        assert_eq!(generate_lfs_server_url(url), LFS_SERVER_URL);

        let url = "https://github.com/web3infra-foundation/mega.git".to_owned();
        assert_eq!(generate_lfs_server_url(url), LFS_SERVER_URL);

        let url = "git@github.com:web3infra-foundation/mega.git".to_owned();
        assert_eq!(generate_lfs_server_url(url), LFS_SERVER_URL);

        let url = "ssh://github.com/web3infra-foundation/mega.git".to_owned();
        assert_eq!(generate_lfs_server_url(url), LFS_SERVER_URL);
    }

    #[test]
    fn test_gen_mono_lfs_server_url() {
        const LFS_SERVER_URL: &str = "https://gitmono.com/web3infra-foundation/mega.git/info/lfs";
        assert_eq!(
            generate_lfs_server_url(LFS_SERVER_URL.to_owned()),
            "https://gitmono.com"
        );
        const LOCAL_LFS_SERVER_URL: &str = "http://localhost:8000/xxx/yyy";
        assert_eq!(
            Url::parse(LOCAL_LFS_SERVER_URL).unwrap().domain().unwrap(),
            "localhost"
        );
        assert_eq!(
            generate_lfs_server_url(LOCAL_LFS_SERVER_URL.to_owned()),
            "http://localhost:8000"
        );
    }

    #[test]
    fn test_parse_pointer_data() {
        let data = r#"version https://git-lfs.github.com/spec/v1
oid sha256:4859402c258b836d02e955d1090e29f586e58b2040504d68afec3d8d43757bba
size 10
"#;
        let res = parse_pointer_data(data.as_bytes()).unwrap();
        println!("{res:?}");
        assert_eq!(
            res.0,
            "4859402c258b836d02e955d1090e29f586e58b2040504d68afec3d8d43757bba"
        );
        assert_eq!(res.1, 10);
    }
}
