use super::ProtocolClient;
use ceres::protocol::smart::read_pkt_line;
use reqwest::Client;
use url::Url;

/// A Git protocol client that communicates with a Git server over HTTPS.
/// Only support `SmartProtocol` now, see https://www.git-scm.com/docs/http-protocol for protocol details.
pub struct HttpsClient {
    url: url::Url,
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
        Self { url }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredReference {
    pub(crate) hash: String,
    pub(crate) _ref: String, // TODO rename to ref
}

#[allow(dead_code)] // todo: unimplemented
impl HttpsClient {
    /// GET $GIT_URL/info/refs?service=git-upload-pack HTTP/1.0
    /// discover the references of the remote repository before fetching the objects.
    /// the first ref named HEAD as default ref.
    pub async fn discovery_reference(
        &self,
    ) -> Result<Vec<DiscoveredReference>, Box<dyn std::error::Error>> {
        let url = self.url.join("info/refs?service=git-upload-pack").unwrap();
        let client = Client::builder().http1_only().build().unwrap();
        let res = client.get(url).send().await.unwrap();
        tracing::debug!("{:?}", res);

        // check Content-Type MUST be application/x-$servicename-advertisement
        let content_type = res.headers().get("Content-Type").unwrap();
        if content_type.to_str().unwrap() != "application/x-git-upload-pack-advertisement" {
            return Err("Error Response format, content_type didn't match `application/x-git-upload-pack-advertisement`".into());
        }

        // check status code MUST be 200 or 304
        // assert!(res.status() == 200 || res.status() == 304);
        if res.status() != 200 && res.status() != 304 {
            return Err(format!("Error Response format, status code: {}", res.status()).into());
        }

        let mut response_content = res.bytes().await.unwrap();
        tracing::debug!("{:?}", response_content);

        // the first five bytes of the response entity matches the regex ^[0-9a-f]{4}#.
        // verify the first pkt-line is # service=$servicename, and ignore LF
        let (_, first_line) = read_pkt_line(&mut response_content);
        if first_line[..].ne(b"# service=git-upload-pack\n") {
            return Err(
                "Error Response format, didn't start with `# service=git-upload-pack`".into(),
            );
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
                // ..default ref named HEAD as the first ref. The stream MUST include capability declarations behind a NUL on the first ref.
                ref_list.push(DiscoveredReference {
                    hash: hash.to_string(),
                    _ref: "HEAD".to_string(),
                });
                let (head, caps) = refs.split_once('\0').unwrap();
                assert_eq!(head, "HEAD");
                let caps = caps.split(' ').collect::<Vec<&str>>();
                // TODO why println will output after all tracing::debug!?
                tracing::debug!("capability declarations: {:?}", caps);
                // tracing::warn!(
                //     "temporary ignore capability declarations:[ {:?} ]",
                //     refs[4..].to_string()
                // );
                read_first_line = true;
            } else {
                ref_list.push(DiscoveredReference {
                    hash: hash.to_string(),
                    _ref: refs.to_string(),
                });
            }
        }
        Ok(ref_list)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use tokio::io::AsyncReadExt;
    use crate::internal::protocel::test::{init_debug_loger, init_loger};

    use tokio_util::io::StreamReader;
    use tokio::io::AsyncBufReadExt;
    use futures_util::TryStreamExt;

    use super::*;

    #[tokio::test]
    async fn test_get_git_upload_pack() {
        init_debug_loger();
        // GitHub may be harder to connect than Gitee
        let test_repo = "https://github.com/web3infra-foundation/mega.git/";

        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference().await;
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
        init_loger();

        // POST $GIT_URL/git-upload-pack HTTP/1.0
        let test_repo = "https://gitee.com/caiqihang2024/image-viewer2.0.git/";

        let url = Url::parse(test_repo)
            .unwrap()
            .join("git-upload-pack")
            .unwrap();

        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference().await.unwrap();
        let refs: Vec<DiscoveredReference> = refs
            .iter()
            .filter(|r| r._ref.starts_with("refs/heads"))
            .cloned()
            .collect();
        println!("{:?}", refs);

        let client = Client::builder().http1_only().build().unwrap();
        let mut body = String::new();
        // body += format!("0032want {}\n", refs[0].hash).as_str();
        for r in refs {
            body += format!("0032want {}\n", r.hash).as_str();
        }
        body += "00000009done\n"; // '\n' is important or no response!
        println!("body:\n{}\n", body);
        let res = client
            .post(url)
            .header("Content-Type", "application/x-git-upload-pack-request")
            .body(body)
            .send()
            .await
            .unwrap();
        println!("{:?}", res.status());

        // let b = &res.bytes().await.unwrap()[0..100];
        // println!("{:?}", b.to_vec());
        // println!("{:?}", res.bytes().await.unwrap()[0..100]);

        // todo: status code 200 but response body is empty

        if res.status().is_success() {
            let stream = res.bytes_stream().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
            let mut reader = StreamReader::new(stream);
            let mut line = String::new();

            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "0008NAK\n");
            println!("First line: {}", line);

            // 创建一个文件并获取写入器
            let mut file = std::fs::File::create("/tmp/pack").unwrap();

            // 将 StreamReader 包装成 Vec<u8> 以便写入文件
            let mut buffer: Vec<u8> = Vec::new();
            loop {
                let mut temp_buffer = [0; 1024];
                let n = match reader.read(&mut temp_buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => n,
                    Err(e) => panic!("error reading from socket; error = {:?}", e)
                };

                buffer.extend_from_slice(&temp_buffer[..n]);
            }

            // 将剩余的数据写入文件
            file.write_all(&buffer).expect("write failed");
        }
    }
}
