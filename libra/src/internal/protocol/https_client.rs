use super::ProtocolClient;
use bytes::Bytes;
use ceres::protocol::smart::{add_pkt_line_string, read_pkt_line};
use futures_util::{StreamExt, TryStreamExt};
use std::io::Error as IoError;
use tokio_util::bytes::BytesMut;
use url::Url;
use ceres::protocol::ServiceType;
use ceres::protocol::ServiceType::UploadPack;
use mercury::errors::GitError;

/// A Git protocol client that communicates with a Git server over HTTPS.
/// Only support `SmartProtocol` now, see https://www.git-scm.com/docs/http-protocol for protocol details.
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
    /// GET $GIT_URL/info/refs?service=git-upload-pack HTTP/1.0
    /// discover the references of the remote repository before fetching the objects.
    /// the first ref named HEAD as default ref.
    /// ## Args
    /// - auth: (username, password)
    pub async fn discovery_reference(
        &self, service: ServiceType, auth: Option<(String, Option<String>)>
    ) -> Result<Vec<DiscRef>, GitError> {
        let service: &str = &service.to_string();
        let url = self.url.join(&format!("info/refs?service={}", service)).unwrap();
        let mut request = self.client.get(url);
        if let Some(auth) = auth {
            request = request.basic_auth(auth.0, auth.1);
        }
        let res = request.send().await.unwrap();
        tracing::debug!("{:?}", res);

        if res.status() == 401 {
            return Err(GitError::UnAuthorized("May need to provide username and password".to_string()));
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
        if content_type != format!("application/x-{}-advertisement", service) {
            return Err(GitError::NetworkError(
                format!("Content-type must be `application/x-{}-advertisement`, but got: {}", service, content_type)));
        }

        let mut response_content = res.bytes().await.unwrap();
        tracing::debug!("{:?}", response_content);

        // the first five bytes of the response entity matches the regex ^[0-9a-f]{4}#.
        // verify the first pkt-line is # service=$servicename, and ignore LF
        let (_, first_line) = read_pkt_line(&mut response_content);
        if first_line[..].ne(format!("# service={}\n", service).as_bytes()) {
            return Err(GitError::NetworkError(
                format!("Error Response format, didn't start with `# service={}`", service)));
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
                let (head, caps) = refs.split_once('\0').unwrap();
                if service == UploadPack.to_string() {
                    // for git-upload-pack, the first line is HEAD
                    assert_eq!(head, "HEAD");
                }
                // ..default ref named HEAD as the first ref. The stream MUST include capability declarations behind a NUL on the first ref.
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

    /// POST $GIT_URL/git-upload-pack HTTP/1.0
    /// Fetch the objects from the remote repository, which is specified by `have` and `want`.
    /// `have` is the list of objects' hashes that the client already has, and `want` is the list of objects that the client wants.
    /// Obtain the `want` references from the `discovery_reference` method.
    /// If the returned stream is empty, it may be due to incorrect refs or an incorrect format.
    // TODO support some necessary options
    pub async fn fetch_objects(
        &self,
        have: &Vec<String>,
        want: &Vec<String>,
    ) -> Result<impl StreamExt<Item = Result<Bytes, IoError>>, IoError> {
        // POST $GIT_URL/git-upload-pack HTTP/1.0
        let url = self.url.join("git-upload-pack").unwrap();
        let mut buf = BytesMut::new();
        let mut write_first_line = false;

        for w in want {
            // body += format!("0032want {}\n", w).as_str();
            if !write_first_line {
                add_pkt_line_string(&mut buf, format!("want {}\0multi_ack_detailed side-band-64k thin-pack ofs-delta agent=libra/0.1.0\n", w).to_string());
                write_first_line = true;
            } else {
                add_pkt_line_string(&mut buf, format!("want {}\n", w).to_string());
            }
        }
        for h in have {
            add_pkt_line_string(&mut buf, format!("have {}\n", h).to_string());
        }

        buf.extend(b"0000"); // split pkt-lines with a flush-pkt
        add_pkt_line_string(&mut buf, "done\n".to_string());

        let body = buf.freeze();
        tracing::debug!("fetch_objects with body:\n{:?}", body);

        let res = self
            .client
            .post(url)
            .header("Content-Type", "application/x-git-upload-pack-request")
            .body(body)
            .send()
            .await
            .unwrap();

        if res.status() != 200 && res.status() != 304 {
            tracing::error!("request failed: {:?}", res);
            return Err(IoError::new(
                std::io::ErrorKind::Other,
                format!("Error Response format, status code: {}", res.status()),
            ));
        }
        // return Ok(res.bytes_stream());
        let result = res
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use crate::utils::test::{init_debug_logger, init_logger};
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncReadExt;
    use tokio_util::io::StreamReader;
    use mercury::hash::SHA1;
    use crate::command::ask_username_password;

    use super::*;

    #[tokio::test]
    async fn test_get_git_upload_pack() {
        init_debug_logger();

        let test_repo = "https://github.com/web3infra-foundation/mega.git/";

        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference(UploadPack, None).await;
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
    async fn test_post_git_upload_pack() {
        init_logger();

        let test_repo = "https://gitee.com/caiqihang2024/image-viewer2.0.git/";

        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference(UploadPack, None).await.unwrap();
        let refs: Vec<DiscoveredReference> = refs
            .iter()
            .filter(|r| r._ref.starts_with("refs/heads"))
            .cloned()
            .collect();
        println!("{:?}", refs);

        let want = refs.iter().map(|r| r._hash.clone()).collect();
        let result_stream = client.fetch_objects(&vec![], &want).await.unwrap();

        let mut reader = StreamReader::new(result_stream);
        let mut line = String::new();

        reader.read_line(&mut line).await.unwrap();
        assert_eq!(line, "0008NAK\n");
        tracing::info!("First line: {}", line);

        let mut buffer = Vec::new();
        loop {
            let mut temp_buffer = [0; 1024];
            let n = match reader.read(&mut temp_buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => panic!("error reading from socket; error = {:?}", e),
            };

            buffer.extend_from_slice(&temp_buffer[..n]);
        }
        tracing::info!("buffer len: {:?}", buffer.len());
        assert!(!buffer.is_empty(), "buffer len is 0, fetch_objects failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_push_empty_repo() {
        init_logger();

        let test_repo = "https://gitee.com/caiqihang2024/test-git-remote.git/";
        let pack_file = r"xxx.pack";

        let mut buf = BytesMut::new();
        add_pkt_line_string(&mut buf, format!("{} {} {}\n", //\0 report-status
                            SHA1::default().to_plain_str(),
                            "d8bd0a95f4fb431e64fcd91098d47a008d7eec4c",
                            "refs/heads/master"));

        buf.extend(b"0000");
        let pack_content = fs::read(pack_file).unwrap();
        buf.extend(pack_content);
        println!("{:?}", buf);

        let (username, password) = ask_username_password();
        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let res = client
            .client
            .post(client.url.join("git-receive-pack").unwrap())
            .header("Content-Type", "application/x-git-receive-pack-request")
            .basic_auth(username, Some(password))
            .body(buf.freeze())
            .send()
            .await
            .unwrap();
        println!("{:?}", res);
    }
}
