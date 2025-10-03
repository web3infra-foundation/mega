use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{utils, ScorpioLFS};
use crate::internal::protocol::LFSClient;
use crate::util;
use crate::utils::lfs;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use wax::Pattern;

// 使用 Lazy 和 Mutex 实现单例模式
pub static LFS_PATTERNSTON: Lazy<Arc<Mutex<Vec<String>>>> = Lazy::new(|| {
    // 在初始化时调用 add_lfs_patterns 函数
    let patterns = extract_lfs_patterns(utils::lfs_attribate().to_str().unwrap()).unwrap();
    Arc::new(Mutex::new(patterns)) // 确保返回的是 Mutex 类型
});
pub async fn add_lfs_patterns(file_path: &str, patterns: Vec<String>) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(file_path)?;

    if file.metadata()?.len() > 0 {
        file.seek(SeekFrom::End(-1))?;

        let mut last_byte = [0; 1];
        file.read_exact(&mut last_byte)?;

        // ensure the last byte is '\n'
        if last_byte[0] != b'\n' {
            file.write_all(b"\n")?;
        }
    }

    let lfs_patterns = extract_lfs_patterns(file_path)?;
    for pattern in patterns {
        if lfs_patterns.contains(&pattern) {
            continue;
        }
        println!("Tracking \"{pattern}\"");
        let pattern = format!(
            "{} filter=lfs diff=lfs merge=lfs -text\n",
            pattern.replace(" ", r"\ ")
        );
        file.write_all(pattern.as_bytes())?;
    }

    file.flush()?;
    let _ = update_global_lfs_patterns(file_path).await;

    Ok(())
}
/// Extract LFS patterns from `.libra_attributes` file
pub fn extract_lfs_patterns<P>(file_path: P) -> std::io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let path = file_path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // ' ' needs '\' before it to be escaped
    let re = regex::Regex::new(r"^\s*(([^\s#\\]|\\ )+)").unwrap();

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

pub async fn untrack_lfs_patterns(file_path: &str, patterns: Vec<String>) -> std::io::Result<()> {
    if !Path::new(file_path).exists() {
        return Ok(());
    }
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let mut matched_pattern = None;
        // delete the specified lfs patterns
        for pattern in &patterns {
            let pattern = pattern.replace(" ", r"\ ");
            if line.trim_start().starts_with(&pattern) && line.contains("filter=lfs") {
                matched_pattern = Some(pattern);
                break;
            }
        }
        match matched_pattern {
            Some(pattern) => println!("Untracking \"{pattern}\""),
            None => lines.push(line),
        }
    }

    // clear the file
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;

    for line in lines {
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
    }
    file.flush()?;
    let _ = update_global_lfs_patterns(file_path).await;
    Ok(())
}

async fn update_global_lfs_patterns(file_path: &str) -> std::io::Result<()> {
    let new_patterns = extract_lfs_patterns(file_path)?;
    let mut patterns = LFS_PATTERNSTON.lock().await;
    *patterns = new_patterns;
    Ok(())
}

/// - absolute path
pub async fn is_lfs_tracked<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    let lfs_pattern = LFS_PATTERNSTON.lock().await;

    let path = util::to_workdir_path(path);
    let glob = wax::any(lfs_pattern.iter().map(|s| s.as_str()).collect::<Vec<_>>()).unwrap();
    glob.is_match(path.to_str().unwrap())
}

/// Generate LFS cache path, in `.libra/lfs/objects`
pub fn lfs_object_path(oid: &str) -> PathBuf {
    utils::lfs_path()
        .join("objects")
        .join(&oid[..2])
        .join(&oid[2..4])
        .join(oid)
}

pub async fn lfs_restore(mono_path: &str, lower_path: &str) -> std::io::Result<()> {
    let lfs_client = LFSClient::scorpio_new(mono_path);
    for entry in ignore::Walk::new(lower_path).filter_map(|e| e.ok()) {
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            let path_str = entry.path().to_str().unwrap();
            if is_lfs_tracked(path_str).await {
                let pointer_bytes = std::fs::read(path_str).expect("Failed to read file");
                let (oid, size) = lfs::parse_pointer_data(&pointer_bytes).unwrap(); // parse pointer data
                                                                                    // LFS file
                let lfs_obj_path = lfs_object_path(&oid);
                if lfs_obj_path.exists() {
                    // found in local cache
                    std::fs::copy(&lfs_obj_path, path_str)?;
                } else {
                    // not exist, download from server
                    if let Err(e) = lfs_client.download_object(&oid, size, path_str).await {
                        eprintln!("LFS Download fatal: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}
#[allow(dead_code)]
pub fn get_oid_by_path(_path: &str) -> String {
    todo!() // create a old lfs pointer storage.
}

/// Copy LFS file to `.libra/lfs/objects`
/// - absolute path
pub fn backup_lfs_file<P>(path: P, oid: &str) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let backup_path = lfs_object_path(oid);
    if !backup_path.exists() {
        std::fs::create_dir_all(backup_path.parent().unwrap())?;
        std::fs::copy(path, backup_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use crate::internal::protocol::{LFSClient, ProtocolClient};
    use mercury::internal::{object::blob::Blob, pack::entry::Entry};

    use crate::scolfs::{ext::BlobExt, ScorpioLFS};

    use super::utils;

    #[tokio::test]
    async fn test_lfs_patterns() {
        let binding = utils::lfs_attribate();
        let attr_path = binding.to_str().unwrap();
        // Clean up before test
        let _ = std::fs::remove_file(attr_path);

        // Add patterns
        super::add_lfs_patterns(attr_path, vec!["a.txt".to_string(), "*.bin".to_string()])
            .await
            .expect("Failed to add LFS patterns");

        // Check patterns are tracked
        assert!(
            super::is_lfs_tracked("a.txt").await,
            "a.txt should be tracked"
        );
        assert!(
            super::is_lfs_tracked("test.bin").await,
            "test.bin should be tracked"
        );

        // Remove patterns
        super::untrack_lfs_patterns(attr_path, vec!["a.txt".to_string()])
            .await
            .expect("Failed to untrack LFS pattern");

        // Check pattern is untracked
        assert!(
            !super::is_lfs_tracked("a.txt").await,
            "a.txt should not be tracked after untrack"
        );
        assert!(
            super::is_lfs_tracked("test.bin").await,
            "test.bin should still be tracked"
        );
    }

    #[test]
    fn test_lfs_point_file() {
        let temp_dir = Path::new("/tmp/mega");
        std::fs::create_dir_all(temp_dir).expect("Failed to create temp dir");

        let test_bin_path = temp_dir.join("test.bin");
        std::fs::write(&test_bin_path, b"dummy content").expect("Failed to create test.bin");
        // Generate pointer file and oid
        let (pointer, oid) = crate::utils::lfs::generate_pointer_file(test_bin_path);
        println!("pointer: {pointer}, oid: {oid}");

        // Create a dummy file to backup
        let test_file = temp_dir.join("test.bin");
        std::fs::write(&test_file, b"dummy content").expect("Failed to write test file");

        // Backup LFS file
        super::backup_lfs_file(&test_file, &oid).expect("Failed to backup LFS file");

        // Check backup exists
        let backup_path = super::lfs_object_path(&oid);
        assert!(backup_path.exists(), "LFS backup file should exist");
    }

    #[tokio::test]
    #[ignore]
    async fn test_lfs_push() {
        {
            let temp_dir = Path::new("/tmp/mega");
            let url = url::Url::parse("http://47.79.35.136:8000/third-party/mega.git").unwrap();
            let client = LFSClient::from_url(&url);
            let bin_blob = Blob::from_lfs_file(temp_dir.join("test.bin"));
            print!("{bin_blob}");
            let data_string = String::from_utf8(bin_blob.data.clone())
                .expect("Failed to convert bin_blob data to string");
            println!("bin_blob data as string: {data_string}");
            let e = Entry::from(bin_blob);
            let res = client.scorpio_push([e].iter()).await;

            if res.is_err() {
                eprintln!("fatal: LFS files upload failed, stop pushing");
                return;
            }
        }
    }
}
