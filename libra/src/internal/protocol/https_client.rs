use super::{
    DiscRef, FetchStream, ProtocolClient, generate_upload_pack_content, parse_discovered_references,
};
use crate::command::ask_basic_auth;
use ceres::protocol::ServiceType;
use futures_util::{StreamExt, TryStreamExt};
use mercury::errors::GitError;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Body, RequestBuilder, Response, StatusCode};
use std::io::Error as IoError;
use std::ops::Deref;
use std::sync::Mutex;
use url::Url;

/// A Git protocol client that communicates with a Git server over HTTPS.
/// Only support `SmartProtocol` now, see [http-protocol](https://www.git-scm.com/docs/http-protocol) for protocol details.
pub struct HttpsClient {
    pub(crate) url: Url,
    pub(crate) client: reqwest::Client,
}

impl ProtocolClient for HttpsClient {
    fn from_url(url: &Url) -> Self {
        // TODO check repo url
        let url = if url.path().ends_with('/') {
            url.clone()
        } else {
            let mut url = url.clone();
            url.set_path(&format!("{}/", url.path()));
            url
        };
        let client = reqwest::Client::builder().http1_only().build().unwrap();
        Self { url, client }
    }
}

/// simply authentication: `username` and `password`
#[derive(Debug, Clone, PartialEq)]
pub struct BasicAuth {
    pub(crate) username: String,
    pub(crate) password: String,
}
static AUTH: Mutex<Option<BasicAuth>> = Mutex::new(None);
impl BasicAuth {
    /// set username & password manually
    pub async fn set_auth(auth: BasicAuth) {
        AUTH.lock().unwrap().replace(auth);
    }

    /// send request with basic auth, retry 3 times
    pub async fn send<Fut>(request_builder: impl Fn() -> Fut) -> Result<Response, reqwest::Error>
    where
        Fut: std::future::Future<Output = RequestBuilder>,
    {
        const MAX_TRY: usize = 3;
        let mut res;
        let mut try_cnt = 0;
        loop {
            let mut request = request_builder().await; // RequestBuilder can't be cloned
            if let Some(auth) = AUTH.lock().unwrap().deref() {
                request = request.basic_auth(auth.username.clone(), Some(auth.password.clone()));
            } // if no auth exists, try without auth (e.g. clone public)
            res = request.send().await?;
            if res.status() == StatusCode::FORBIDDEN {
                // 403: no access, no need to retry
                eprintln!("Authentication failed, forbidden");
                break;
            } else if res.status() != StatusCode::UNAUTHORIZED {
                break;
            }
            // 401 (Unauthorized): username or password is incorrect
            if try_cnt >= MAX_TRY {
                eprintln!("Failed to authenticate after {MAX_TRY} attempts");
                break;
            }
            eprintln!("Authentication required, retrying...");
            AUTH.lock().unwrap().replace(ask_basic_auth());
            try_cnt += 1;
        }
        Ok(res)
    }
}

// Client communicates with the remote git repository over SMART protocol.
// protocol details: https://www.git-scm.com/docs/http-protocol
// capability declarations: https://www.git-scm.com/docs/protocol-capabilities
impl HttpsClient {
    /// GET $GIT_URL/info/refs?service=git-upload-pack HTTP/1.0<br>
    /// Discover the references of the remote repository before fetching the objects.
    /// the first ref named HEAD as default ref.
    /// ## Args
    /// - auth: (username, password)
    pub async fn discovery_reference(
        &self,
        service: ServiceType,
    ) -> Result<Vec<DiscRef>, GitError> {
        let service_name = service.to_string();
        let url = self
            .url
            .join(&format!("info/refs?service={service_name}"))
            .unwrap();
        let res = BasicAuth::send(|| async { self.client.get(url.clone()) })
            .await
            .map_err(|e| GitError::NetworkError(format!("Failed to send request: {}", e)))?;
        tracing::debug!("{:?}", res);

        if res.status() == 401 {
            return Err(GitError::UnAuthorized(
                "May need to provide username and password".to_string(),
            ));
        }
        // check status code MUST be 200 or 304
        if res.status() != 200 && res.status() != 304 {
            return Err(GitError::NetworkError(format!(
                "Error Response format, status code: {}",
                res.status()
            )));
        }

        // check Content-Type MUST be application/x-$servicename-advertisement
        let content_type = res
            .headers()
            .get("Content-Type")
            .ok_or_else(|| GitError::NetworkError("Missing Content-Type header".to_string()))?
            .to_str()
            .map_err(|e| GitError::NetworkError(format!("Invalid Content-Type header: {}", e)))?;
        if content_type != format!("application/x-{service_name}-advertisement") {
            return Err(GitError::NetworkError(format!(
                "Content-type must be `application/x-{service_name}-advertisement`, but got: {content_type}"
            )));
        }

        let response_content = res
            .bytes()
            .await
            .map_err(|e| GitError::NetworkError(format!("Failed to read response body: {}", e)))?;
        tracing::debug!("{:?}", response_content);

        parse_discovered_references(response_content, service)
    }

    /// POST $GIT_URL/git-upload-pack HTTP/1.0<br>
    /// Fetch the objects from the remote repository, which is specified by `have` and `want`.<br>
    /// `have` is the list of objects' hashes that the client already has, and `want` is the list of objects that the client wants.
    /// Obtain the `want` references from the `discovery_reference` method.<br>
    /// If the returned stream is empty, it may be due to incorrect refs or an incorrect format.
    // TODO support some necessary options
    pub async fn fetch_objects(
        &self,
        have: &[String],
        want: &[String],
    ) -> Result<FetchStream, IoError> {
        // POST $GIT_URL/git-upload-pack HTTP/1.0
        let url = self.url.join("git-upload-pack").unwrap();
        let body = generate_upload_pack_content(have, want);
        tracing::debug!("fetch_objects with body: {:?}", body);

        let res = BasicAuth::send(|| async {
            self.client
                .post(url.clone())
                .header("Content-Type", "application/x-git-upload-pack-request")
                .body(body.clone())
        })
        .await
        .map_err(|e| IoError::other(format!("Failed to send request: {}", e)))?;
        tracing::debug!("request: {:?}", res);

        if res.status() != 200 && res.status() != 304 {
            tracing::error!("request failed: {:?}", res);
            return Err(IoError::other(format!(
                "Error Response format, status code: {}",
                res.status()
            )));
        }
        let result = res.bytes_stream().map_err(std::io::Error::other).boxed();

        Ok(result)
    }

    pub async fn send_pack<T: Into<Body> + Clone>(
        &self,
        data: T,
    ) -> Result<Response, reqwest::Error> {
        BasicAuth::send(|| async {
            self.client
                .post(self.url.join("git-receive-pack").unwrap())
                .header(CONTENT_TYPE, "application/x-git-receive-pack-request")
                .body(data.clone())
        })
        .await
    }
}
#[cfg(test)]
mod tests {

    use crate::utils::test::init_debug_logger;
    use crate::utils::test::init_logger;
    use ceres::protocol::ServiceType::UploadPack;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    use super::*;

    #[tokio::test]
    #[ignore] // This test requires network connectivity
    async fn test_discover_reference_upload() {
        init_debug_logger();

        let test_repo = "https://github.com/web3infra-foundation/mega.git/";

        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference(UploadPack).await;
        if refs.is_err() {
            tracing::error!("{:?}", refs.err().unwrap());
            panic!();
        } else {
            let refs = refs.unwrap();
            println!("refs count: {:?}", refs.len());
            println!("example: {:?}", refs[1]);
        }
    }

    #[tokio::test]
    #[ignore] // This test requires network connectivity
    async fn test_post_git_upload_pack_() {
        init_debug_logger();

        let test_repo = "https://github.com/web3infra-foundation/mega/";
        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference(UploadPack).await.unwrap();
        let refs: Vec<DiscRef> = refs
            .iter()
            .filter(|r| r._ref.starts_with("refs/heads"))
            .cloned()
            .collect();
        tracing::info!("refs: {:?}", refs);

        let want: Vec<String> = refs.iter().map(|r| r._hash.clone()).collect();

        let have = vec!["81a162e7b725bbad2adfe01879fd57e0119406b9".to_string()];
        let mut result_stream = client.fetch_objects(&have, &want).await.unwrap();

        let mut buffer = vec![];
        while let Some(item) = result_stream.next().await {
            let item = item.unwrap();
            buffer.extend(item);
        }

        // pase pkt line
        if let Some(pack_pos) = buffer.windows(4).position(|w| w == b"PACK") {
            tracing::info!("pack data found at: {}", pack_pos);
            let readable_output = std::str::from_utf8(&buffer[..pack_pos]).unwrap();
            tracing::debug!("stdout readable: \n{}", readable_output);
            tracing::info!("pack length: {}", buffer.len() - pack_pos);
            assert!(buffer[pack_pos..pack_pos + 4].eq(b"PACK"));
        } else {
            tracing::error!(
                "no pack data found, stdout is :\n{}",
                std::str::from_utf8(&buffer).unwrap()
            );
            panic!("no pack data found");
        }
    }

    #[tokio::test]
    #[ignore] // ignore this because **user should edit the `want` maurally**
    async fn test_upload_pack_local() {
        // use /usr/bin/git-upload-pack as a test server. if no /usr/bin/git-upload-pack, skip this test
        if !std::path::Path::new("/usr/bin/git-upload-pack").exists() {
            return;
        }
        // init_debug_logger();
        init_logger();

        let have = vec!["1c05d7f7dd70e38150bfd2d5fb8fb969e2eb9851".to_string()];
        // **want MUST change to one of the refs in the remote repo, such as `refs/heads/main` before running the test**
        let want = vec!["6b4e69962dbbc75e80d5263cc5c81571669db9bc".to_string()];
        let body = generate_upload_pack_content(&have, &want);
        tracing::info!("upload-pack content: {:?}", body);
        let mut cmd = tokio::process::Command::new("/usr/bin/git-upload-pack");
        cmd.arg("..");

        // set up the pipe otherwise the `take` will get None
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().unwrap();

        // write the body to the stdin of the child process
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&body).await.unwrap();

        // check the stderr of the child process
        let mut output = child.stderr.take().unwrap();
        let mut stderr = String::new();
        output.read_to_string(&mut stderr).await.unwrap();
        tracing::info!("stderr: {}", stderr);
        assert!(!stderr.contains("protocol error"), "{}", stderr);
        if stderr.contains("not our ref") {
            tracing::error!(
                "not our ref, please change the `want` to one of the refs in the target repo"
            );
            panic!();
        }

        let mut output = child.stdout.take().unwrap();
        let mut stdout = vec![];
        output.read_to_end(&mut stdout).await.unwrap();
        assert!(stdout.len() > 100, "stdout is empty");
        tracing::info!("stdout len: {}", stdout.len());

        if let Some(pack_pos) = stdout.windows(4).position(|w| w == b"PACK") {
            tracing::info!("pack data found at: {}", pack_pos);
            let readable_output = std::str::from_utf8(&stdout[..pack_pos]).unwrap();
            tracing::debug!("stdout readable: \n{}", readable_output);
            tracing::info!("pack length: {}", stdout.len() - pack_pos);
            // assert!(stdout[..4].eq(b"PACK"));
            assert!(stdout[pack_pos..pack_pos + 4].eq(b"PACK"));
        } else {
            tracing::error!(
                "no pack data found, stdout is {}\n",
                std::str::from_utf8(&stdout).unwrap()
            );
            panic!("no pack data found");
        }
    }
}
