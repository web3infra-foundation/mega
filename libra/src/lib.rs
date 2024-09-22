use mercury::errors::GitError;

mod command;
mod internal;
mod utils;
pub mod cli;

/// Execute the Libra command in `sync` way.
/// ### Caution
/// There is a tokio runtime inside. Ensure you are NOT in a tokio runtime which can't be nested.
/// ### Example
/// - `["init"]`
/// - `["add", "."]`
pub fn exec(mut args: Vec<&str>) -> Result<(), GitError> {
    args.insert(0, env!("CARGO_PKG_NAME"));
    cli::parse(Some(&args))
}

/// Execute the Libra command in `async` way.
/// - `async` version of the [exec] function
pub async fn exec_async(mut args: Vec<&str>) -> Result<(), GitError> {
    args.insert(0, env!("CARGO_PKG_NAME"));
    cli::parse_async(Some(&args)).await
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use super::*;

    #[test]
    fn test_libra_init() {
        let tmp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(tmp_dir.path()).unwrap();
        exec(vec!["init"]).unwrap();
    }
}