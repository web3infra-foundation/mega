use url::Url;

pub mod https_client;
pub mod lfs_client;

#[allow(dead_code)] // todo: unimplemented
pub trait ProtocolClient {
    /// create client from url
    fn from_url(url: &Url) -> Self;
}

#[cfg(test)]
mod test {}
