use mercury::errors::GitError;

pub mod cli;
mod command;
pub mod internal;
pub mod utils;

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
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_libra_init() {
        let original_dir = std::env::current_dir().unwrap();

        let tmp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(tmp_dir.path()).unwrap();
        exec(vec!["init"]).unwrap();

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_lfs_client() {
        use crate::internal::protocol::lfs_client::LFSClient;
        use crate::internal::protocol::ProtocolClient;
        use url::Url;

        let client = LFSClient::from_url(&Url::parse("https://git.gitmono.org").unwrap());
        println!("{:?}", client);
        let mut report_fn = |progress: f64| {
            println!("progress: {:.2}%", progress);
            Ok(())
        };
        client
            .download_object(
                "a744b4beab939d899e22c8a070b7041a275582fb942483c9436d455173c7e23d",
                338607424,
                "/home/bean/projects/tmp/Qwen2.5-0.5B-Instruct-Q2_K.gguf",
                Some((&mut report_fn, 0.1)),
            )
            .await
            .expect("Failed to download object");
    }
}
