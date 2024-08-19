use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use regex::Regex;

/// Check if a file is LFS tracked
/// - only check root attributes file now, should check all attributes files in sub-dirs
// pub fn is_lfs_tracked<P>(path: P) -> bool
// where
//     P: AsRef<Path>,
// {
//     // check .libra_attributes
//     let path = path.as_ref();
//
// }

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