use url::Url;

pub mod https_client;

pub trait ProtocolClient {
    /// create client from url
    fn from_url(url: &Url) -> Self;
}