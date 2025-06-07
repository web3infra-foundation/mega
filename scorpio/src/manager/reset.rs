use std::fs;
use std::path::Path;

/// Reset the repository, discarding all changes.
pub fn reset_core(work_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let modified_path = work_path.join("modifiedstore");
    let upper_path = work_path.join("upper");

    fs::remove_dir_all(&modified_path)?;
    fs::remove_dir_all(&upper_path)?;

    fs::create_dir(&upper_path)?;

    Ok(())
}
