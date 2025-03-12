use std::path::Path;
use std::{fs, io};

#[allow(dead_code)]
/// Ensure the file exists, create it(with all parent dirs) if not.
pub fn ensure_created(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        ensure_parent_dirs(path)?;
        fs::File::create(path)?;
    }
    Ok(())
}

/// Ensure the parent dirs of the file exists.
pub fn ensure_parent_dirs(path: impl AsRef<Path>) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[allow(dead_code)]
/// Ensure the file exists and has the specified content.
pub fn ensure_file_content(path: impl AsRef<Path>, content: &str) -> io::Result<()> {
    ensure_parent_dirs(path.as_ref())?;
    fs::write(path, content)
}
