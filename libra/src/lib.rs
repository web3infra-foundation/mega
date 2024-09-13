use mercury::errors::GitError;

mod command;
mod internal;
mod utils;
pub mod cli;

/// Execute the Libra command
/// ### Example
/// - `["init"]`
/// - `["add", "."]`
pub fn exec(mut args: Vec<&str>) -> Result<(), GitError> {
    args.insert(0, "libra");
    cli::parse(Some(&args))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use super::*;

    #[test]
    fn test_libra_init() {
        let tmp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&tmp_dir.path()).unwrap();
        exec(vec!["init"]).unwrap();
    }
}