use super::ProtocolClient;
use reqwest::Client;
use std::io::{BufRead, Read};
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
pub struct DiscoverdReference {
    hash: String,
    refs: String,
}

impl HttpsClient {
    pub async fn discovery_reference(
        &self,
    ) -> Result<Vec<DiscoverdReference>, Box<dyn std::error::Error>> {
        // GET $GIT_URL/info/refs?service=git-upload-pack HTTP/1.0
        let url = self
            .url
            .clone()
            .join("info/refs?service=git-upload-pack")
            .unwrap();
        let client = Client::builder().http1_only().build().unwrap();
        let res = client
            .get(url)
            .header("User-Agent", "git/2.0 (git 2.14.1)")
            .send()
            .await
            .unwrap();

        // check Content-Type MUST be application/x-$servicename-advertisement
        let content_type = res.headers().get("Content-Type").unwrap();
        if content_type.to_str().unwrap() != "application/x-git-upload-pack-advertisement" {
            return Err("Error Response format".into());
        }

        // check status code MUST be 200 or 304
        assert!(res.status() == 200 || res.status() == 304);

        let bytes = res.bytes().await.unwrap().to_vec();
        let mut reader = std::io::Cursor::new(&bytes);

        // the first five bytes of the response entity matches the regex ^[0-9a-f]{4}#.
        let mut first_five_bytes = [0u8; 5];
        let mut magix_check = true;
        reader.read_exact(&mut first_five_bytes).unwrap();
        magix_check = first_five_bytes[0..4]
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase());

        magix_check = first_five_bytes[4] == b'#';
        if !magix_check {
            return Err("Error Response format".into());
        }

        // verify the first pkt-line is # service=$servicename, and ignore LF
        let mut pkt_line = String::new();
        reader.read_line(&mut pkt_line).unwrap();
        pkt_line = pkt_line.trim().to_string();
        if pkt_line.ne("service=git-upload-pack") {
            return Err("Error Response format".into());
        }

        reader.read_line(&mut pkt_line).unwrap(); // option supported, ignore temporarily

        let mut ref_list = vec![];
        loop {
            pkt_line.clear();
            reader.read_line(&mut pkt_line).unwrap();

            pkt_line = pkt_line.trim().to_string();
            if pkt_line.starts_with("0000") {
                break; // end of the response
            }

            let (hash, mut refs) = pkt_line.split_at(44);
            refs = refs.trim();
            if refs.starts_with("refs/pull") {
                // XXX ignore pull request
                continue;
            }
            ref_list.push(DiscoverdReference {
                hash: hash.to_string(),
                refs: refs.to_string(),
            });
        }
        Ok(ref_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_git_upload_pack() {
        let test_repo = "https://github.com/web3infra-foundation/mega.git/";
        let client = HttpsClient::from_url(&Url::parse(test_repo).unwrap());
        let refs = client.discovery_reference().await.unwrap();
        println!("{:?}", refs);
    }
}
