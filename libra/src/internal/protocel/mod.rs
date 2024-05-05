use url::Url;

pub mod https_client;
pub trait ProtocolClient {
    /// create client from url
    fn from_url(url: &Url) -> Self;
}

#[cfg(test)]
mod test {

    pub fn init_debug_loger() {
        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )
        .unwrap();
    }

    pub fn init_loger() {
        tracing_subscriber::fmt().init();
    }
}
