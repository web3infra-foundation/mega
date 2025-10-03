pub mod https_client;
pub mod lfs_client;

pub use https_client::BasicAuth;
pub use lfs_client::{LFSClient, LfsBatchResponse};

pub trait ProtocolClient {
    fn from_url(url: &url::Url) -> Self;
}
