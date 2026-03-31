use std::{
    io::{self, Cursor},
    pin::Pin,
    str::from_utf8,
};

use axum::Error as AxumError;
use bytes::{BufMut, Bytes, BytesMut};
use common::errors::MegaError;
use futures::{Stream, StreamExt};
use reqwest::{Client, Url};
use tokio::io::AsyncRead;
use tokio_util::io::StreamReader;

#[derive(Clone)]
pub struct ThirdPartyClient {
    url: Url,
    client: Client,
}

impl ThirdPartyClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            url: Url::parse(base_url).expect("Invalid URL"),
            client: Client::new(),
        }
    }
}

#[async_trait::async_trait]
pub trait ThirdPartyRepoTrait {
    async fn fetch_refs(&self) -> Result<(String, String), MegaError>;
    async fn fetch_packs(
        &self,
        want: &[String],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>, MegaError>;
}

#[async_trait::async_trait]
impl ThirdPartyRepoTrait for ThirdPartyClient {
    async fn fetch_refs(&self) -> Result<(String, String), MegaError> {
        let request_url = format!("{}/info/refs?service=git-upload-pack", self.url);
        let resp = self
            .client
            .get(request_url)
            .send()
            .await
            .map_err(|e| MegaError::Other(format!("{e}")))?;

        if !resp.status().is_success() {
            return Err(MegaError::Other(format!(
                "Unable to fetch refs, status: {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| MegaError::Other(format!("Unable to parse bytes: {}", e)))?;

        let mut cursor = Cursor::new(bytes);
        let mut head_symref: Option<String> = None;
        let mut head_ref_hash: Option<String> = None;
        let mut candidate_branches: Vec<(String, String)> = Vec::new();
        loop {
            let mut len_buf = [0u8; 4];
            if std::io::Read::read_exact(&mut cursor, &mut len_buf).is_err() {
                break;
            }

            let len_hex = from_utf8(&len_buf).map_err(|e| MegaError::Other(format!("{e}")))?;
            let len =
                u32::from_str_radix(len_hex, 16).map_err(|e| MegaError::Other(format!("{e}")))?;

            if len == 0 {
                continue;
            }

            let mut data = vec![0u8; (len - 4) as usize];
            std::io::Read::read_exact(&mut cursor, &mut data)?;
            let line = String::from_utf8_lossy(&data);
            let line = line.trim_end_matches('\n');

            // Each advertised ref is usually: "<hash> <refname>\0<capabilities...>"
            // Only the first advertised ref line contains capabilities.
            let (left, caps) = match line.split_once('\0') {
                Some((l, c)) => (l, Some(c)),
                None => (line, None),
            };

            if let Some(caps) = caps {
                // Parse default branch from symref, e.g. "symref=HEAD:refs/heads/main"
                // Capabilities are separated by spaces.
                for cap in caps.split_whitespace() {
                    if let Some(v) = cap.strip_prefix("symref=HEAD:") {
                        head_symref = Some(v.to_string());
                        break;
                    }
                }
            }

            // Parse "<hash> <refname>" portion (ignore service header lines like "# service=...")
            let mut it = left.split_whitespace();
            let hash = it.next();
            let ref_name = it.next();
            if let (Some(hash), Some(ref_name)) = (hash, ref_name) {
                if ref_name == "HEAD" {
                    head_ref_hash = Some(hash.to_string());
                    continue;
                }
                if ref_name.starts_with("refs/heads/") {
                    candidate_branches.push((ref_name.to_string(), hash.to_string()));
                }
            }
        }

        // 1) Prefer server-advertised default branch via symref=HEAD:refs/heads/<x>
        if let Some(default_ref) = head_symref
            && let Some((r, cmt)) = candidate_branches
                .iter()
                .find(|(r, _)| r == &default_ref)
                .cloned()
        {
            return Ok((r, cmt));
        }

        // 2) Fallback: common defaults
        for preferred in ["refs/heads/main", "refs/heads/master"] {
            if let Some((r, cmt)) = candidate_branches
                .iter()
                .find(|(r, _)| r == preferred)
                .cloned()
            {
                return Ok((r, cmt));
            }
        }

        // 3) Fallback: if HEAD hash is known, pick any branch pointing at HEAD
        if let Some(head_hash) = head_ref_hash
            && let Some((r, cmt)) = candidate_branches
                .iter()
                .find(|(_, h)| h == &head_hash)
                .cloned()
        {
            return Ok((r, cmt));
        }

        // 4) Last resort: first advertised branch
        candidate_branches
            .into_iter()
            .next()
            .ok_or_else(|| MegaError::Other("No refs/heads/* found".to_string()))
    }

    async fn fetch_packs(
        &self,
        want: &[String],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>, MegaError> {
        let request_url = format!("{}/git-upload-pack", self.url);
        let body = self.generate_upload_pack_content(want);
        tracing::debug!("fetch_objects with body {:?}", body);

        let res = self
            .client
            .post(request_url)
            .header("Content-Type", "application/x-git-upload-pack-request")
            .body(body)
            .send()
            .await
            .map_err(|e| MegaError::Other(format!("Failed to send request: {}", e)))?;

        Ok(res.bytes_stream().boxed())
    }
}

impl ThirdPartyClient {
    fn generate_upload_pack_content(&self, want: &[String]) -> Bytes {
        let mut buf = BytesMut::new();
        let mut write_first_line = false;

        let capability = ["side-band-64k", "ofs-delta", "multi_ack_detailed"].join(" ");
        for w in want {
            if !write_first_line {
                self.add_pkt_line_string(
                    &mut buf,
                    format!("want {w} {capability} agent=libra/0.1.0\n"),
                );
                write_first_line = true;
            } else {
                self.add_pkt_line_string(&mut buf, format!("want {w}\n"));
            }
        }
        buf.extend(b"0000");
        self.add_pkt_line_string(&mut buf, "done\n".to_string());

        buf.freeze()
    }

    fn add_pkt_line_string(&self, pkt_line_stream: &mut BytesMut, buf_str: String) {
        let buf_str_length = buf_str.len() + 4;
        pkt_line_stream.put(Bytes::from(format!("{:04x}", buf_str_length)));
        pkt_line_stream.put(buf_str.as_bytes());
    }

    pub async fn process_pack_stream(
        &self,
        res: impl Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
    ) -> Result<Vec<u8>, AxumError> {
        let stream = res.map(|r| r.map_err(|e| io::Error::other(format!("reqwest error: {e}"))));

        let mut reader = StreamReader::new(stream);

        let mut pack_data = Vec::new();
        let mut reach_pack = false;

        loop {
            let (len, data) = match self.read_pkt_line(&mut reader).await {
                Ok(d) => d,
                Err(_) => break,
            };

            if len == 0 {
                break;
            }

            if data.len() >= 5 && &data[1..5] == b"PACK" {
                reach_pack = true;
                tracing::debug!("Receiving PACK data...");
            }

            if reach_pack {
                let code = data[0];
                let data = &data[1..];
                match code {
                    1 => pack_data.extend_from_slice(data),
                    2 => tracing::info!("{}", String::from_utf8_lossy(data)),
                    3 => tracing::warn!("{}", String::from_utf8_lossy(data)),
                    _ => tracing::warn!("unknown side-band-64k code: {code}"),
                }
            } else if &data != b"NAK\n" {
                tracing::info!("{}", String::from_utf8_lossy(&data));
            }
        }
        if pack_data.is_empty() {
            tracing::warn!("no PACK data received");
        }

        Ok(pack_data)
    }

    async fn read_hex_4(&self, reader: &mut (impl AsyncRead + Unpin)) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        tokio::io::AsyncReadExt::read_exact(reader, &mut buf).await?;
        let hex_str =
            std::str::from_utf8(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        u32::from_str_radix(hex_str, 16).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_pkt_line(
        &self,
        reader: &mut (impl AsyncRead + Unpin),
    ) -> io::Result<(usize, Vec<u8>)> {
        let len = self.read_hex_4(reader).await?;
        if len == 0 {
            return Ok((0, Vec::new()));
        }

        let mut data = vec![0u8; (len - 4) as usize];
        tokio::io::AsyncReadExt::read_exact(reader, &mut data).await?;
        Ok((len as usize, data))
    }
}
