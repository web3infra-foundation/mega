use reqwest::{RequestBuilder, Response};
use std::ops::Deref;
use std::sync::Mutex;

/// simply authentication: `username` and `password`
#[derive(Debug, Clone, PartialEq)]
pub struct BasicAuth {
    pub(crate) username: String,
    pub(crate) password: String,
}

static AUTH: Mutex<Option<BasicAuth>> = Mutex::new(None);

impl BasicAuth {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    /// set username & password manually
    pub async fn set_auth(auth: BasicAuth) {
        AUTH.lock().unwrap().replace(auth);
    }

    /// send request with basic auth (simplified version)
    pub async fn send<Fut>(request_builder: impl Fn() -> Fut) -> Result<Response, reqwest::Error>
    where
        Fut: std::future::Future<Output = RequestBuilder>,
    {
        let mut request = request_builder().await;
        if let Some(auth) = AUTH.lock().unwrap().deref() {
            request = request.basic_auth(auth.username.clone(), Some(auth.password.clone()));
        }
        request.send().await
    }
}
