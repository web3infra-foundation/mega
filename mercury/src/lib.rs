//! Mercury is a library for encoding and decoding Git Pack format files or streams.

pub mod errors;
pub mod hash;
pub mod internal;
pub mod utils;

// #[cfg(test)]
pub mod test_utils {
    use reqwest::Client;
    use ring::digest::{Context, SHA256};
    use std::env;
    use std::fs::File;
    use std::io::copy;
    use std::path::PathBuf;
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    async fn calculate_checksum(file: &mut tokio::fs::File, checksum: &mut Context) {
        file.seek(tokio::io::SeekFrom::Start(0)).await.unwrap();
        let mut buf = [0u8; 8192];
        loop {
            let n = file.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            checksum.update(&buf[..n]);
        }
    }

    async fn download_file(url: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = Client::new().get(url).send().await?;
        let mut file = File::create(output_path)?;
        let content = response.bytes().await?;
        let mut content = content.as_ref();
        copy(&mut content, &mut file)?;
        Ok(())
    }

    async fn check_file_hash(output_path: &str, sha256: &str) -> bool {
        let path = PathBuf::from(output_path);
        if path.exists() {
            let mut ring_context = Context::new(&SHA256);
            let mut file = tokio::fs::File::open(output_path).await.unwrap();
            calculate_checksum(&mut file, &mut ring_context).await;
            let checksum = hex::encode(ring_context.finish().as_ref());
            checksum == sha256
        } else {
            false
        }
    }

    async fn download_lfs_file_if_not_exist(file_name: &str, sha256: &str) {
        let url = format!(
            "https://gitmono.s3.ap-southeast-2.amazonaws.com/packs/{}",
            file_name
        );
        let mut cargo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        cargo_path.pop();
        let output_path = format!(
            "{}/tests/data/packs/{}",
            cargo_path.to_str().unwrap(),
            file_name
        );
        if !check_file_hash(&output_path, sha256).await {
            let result = download_file(&url, &output_path).await;
            assert!(result.is_ok());
        }
    }

    pub async fn setup_lfs_file() {
        download_lfs_file_if_not_exist(
            "git-2d187177923cd618a75da6c6db45bb89d92bd504.pack",
            "0d1f01ac02481427e83ba6c110c7a3a75edd4264c5af8014d12d6800699c8409",
        )
        .await;
    }
}
