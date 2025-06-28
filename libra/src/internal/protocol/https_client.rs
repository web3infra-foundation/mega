use super::ProtocolClient;
use crate::command::ask_basic_auth;
use bytes::Bytes;
use ceres::protocol::smart::{add_pkt_line_string, read_pkt_line};
use ceres::protocol::ServiceType;
use ceres::protocol::ServiceType::UploadPack;
use futures_util::{StreamExt, TryStreamExt};
use mercury::errors::GitError;
use mercury::hash::SHA1;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Body, RequestBuilder, Response, StatusCode};
use std::io::Error as IoError;
use std::ops::Deref;
use std::sync::Mutex;
use tokio_util::bytes::BytesMut;
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

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredReference {
    pub(crate) _hash: String,
    pub(crate) _ref: String,
}

type DiscRef = DiscoveredReference;

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
        let service: &str = &service.to_string();
        let url = self
            .url
            .join(&format!("info/refs?service={service}"))
            .unwrap();
        let res = BasicAuth::send(|| async { self.client.get(url.clone()) })
            .await
            .unwrap();
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
        let content_type = res.headers().get("Content-Type").unwrap().to_str().unwrap();
        if content_type != format!("application/x-{service}-advertisement") {
            return Err(GitError::NetworkError(format!(
                "Content-type must be `application/x-{service}-advertisement`, but got: {content_type}"
            )));
        }

        let mut response_content = res.bytes().await.unwrap();
        tracing::debug!("{:?}", response_content);

        // the first five bytes of the response entity matches the regex ^[0-9a-f]{4}#.
        // verify the first pkt-line is # service=$servicename, and ignore LF
        let (_, first_line) = read_pkt_line(&mut response_content);
        if first_line[..].ne(format!("# service={service}\n").as_bytes()) {
            return Err(GitError::NetworkError(format!(
                "Error Response format, didn't start with `# service={service}`"
            )));
        }

        let mut ref_list = vec![];
        let mut read_first_line = false;
        loop {
            let (bytes_take, pkt_line) = read_pkt_line(&mut response_content);
            if bytes_take == 0 {
                if response_content.is_empty() {
                    break;
                } else {
                    continue;
                }
            }
            let pkt_line = String::from_utf8(pkt_line.to_vec()).unwrap();
            let (hash, mut refs) = pkt_line.split_at(40); // hex SHA1 string is 40 bytes
            refs = refs.trim();
            if !read_first_line {
                if hash == SHA1::default().to_string() {
                    break; // empty repo, return empty list // TODO: parse capability
                }
                let (head, caps) = refs.split_once('\0').unwrap();
                if service == UploadPack.to_string() {
                    // for git-upload-pack, the first line is HEAD
                    assert_eq!(head, "HEAD");
                }
                // default ref named HEAD as the first ref. The stream MUST include capability declarations behind a NUL on the first ref.
                ref_list.push(DiscoveredReference {
                    _hash: hash.to_string(),
                    _ref: head.to_string(),
                });
                let caps = caps.split(' ').collect::<Vec<&str>>();
                tracing::debug!("capability declarations: {:?}", caps);
                // tracing::warn!(
                //     "temporary ignore capability declarations:[ {:?} ]",
                //     refs[4..].to_string()
                // );
                read_first_line = true;
            } else {
                ref_list.push(DiscoveredReference {
                    _hash: hash.to_string(),
                    _ref: refs.to_string(),
                });
            }
        }
        Ok(ref_list)
    }

    /// POST $GIT_URL/git-upload-pack HTTP/1.0<br>
    /// Fetch the objects from the remote repository, which is specified by `have` and `want`.<br>
    /// `have` is the list of objects' hashes that the client already has, and `want` is the list of objects that the client wants.
    /// Obtain the `want` references from the `discovery_reference` method.<br>
    /// If the returned stream is empty, it may be due to incorrect refs or an incorrect format.
    // TODO support some necessary options
    pub async fn fetch_objects(
        &self,
        have: &Vec<String>,
        want: &Vec<String>,
    ) -> Result<impl StreamExt<Item = Result<Bytes, IoError>>, IoError> {
        // POST $GIT_URL/git-upload-pack HTTP/1.0
        let url = self.url.join("git-upload-pack").unwrap();
        let body = generate_upload_pack_content(have, want).await;
        tracing::debug!("fetch_objects with body: {:?}", body);

        let res = BasicAuth::send(|| async {
            self.client
                .post(url.clone())
                .header("Content-Type", "application/x-git-upload-pack-request")
                .body(body.clone())
        })
        .await
        .unwrap();
        tracing::debug!("request: {:?}", res);

        if res.status() != 200 && res.status() != 304 {
            tracing::error!("request failed: {:?}", res);
            return Err(IoError::other(format!(
                "Error Response format, status code: {}",
                res.status()
            )));
        }
        let result = res.bytes_stream().map_err(std::io::Error::other);

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
/// for fetching
async fn generate_upload_pack_content(have: &Vec<String>, want: &Vec<String>) -> Bytes {
    let mut buf = BytesMut::new();
    let mut write_first_line = false;

    let capability = ["side-band-64k", "ofs-delta", "multi_ack_detailed"].join(" ");
    for w in want {
        if !write_first_line {
            add_pkt_line_string(
                &mut buf,
                format!("want {w} {capability} agent=libra/0.1.0\n").to_string(),
            );
            write_first_line = true;
        } else {
            add_pkt_line_string(&mut buf, format!("want {w}\n").to_string());
        }
    }
    buf.extend(b"0000"); // split pkt-lines with a flush-pkt
    for h in have {
        add_pkt_line_string(&mut buf, format!("have {h}\n").to_string());
    }

    add_pkt_line_string(&mut buf, "done\n".to_string());

    buf.freeze()
}

#[cfg(test)]
mod tests {

    use crate::utils::test::init_debug_logger;
    use crate::utils::test::init_logger;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    use super::*;

    #[tokio::test]
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
    async fn test_post_git_upload_pack_() {
        init_debug_logger();

        let test_repo = "https://github.com/web3infra-foundation/mega/";
        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference(UploadPack).await.unwrap();
        let refs: Vec<DiscoveredReference> = refs
            .iter()
            .filter(|r| r._ref.starts_with("refs/heads"))
            .cloned()
            .collect();
        tracing::info!("refs: {:?}", refs);

        let want = refs.iter().map(|r| r._hash.clone()).collect();

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
        let body = generate_upload_pack_content(&have, &want).await;
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
