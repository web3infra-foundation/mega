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
    async fn fetch_refs(&self) -> Result<Vec<String>, MegaError>;
    async fn fetch_packs(
        &self,
        want: &[String],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>, MegaError>;
}

#[async_trait::async_trait]
impl ThirdPartyRepoTrait for ThirdPartyClient {
    async fn fetch_refs(&self) -> Result<Vec<String>, MegaError> {
        let request_url = format!("{}/info/refs?service=git-upload-pack", self.url);
        let resp = self
            .client
            .get(request_url)
            .send()
            .await
            .map_err(|e| MegaError::with_message(format!("{e}")))?;

        if !resp.status().is_success() {
            return Err(MegaError::with_message(format!(
                "Unable to fetch refs, status: {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| MegaError::with_message(format!("Unable to parse bytes: {}", e)))?;

        let mut cursor = Cursor::new(bytes);
        let mut refs = Vec::new();

        loop {
            let mut len_buf = [0u8; 4];
            if std::io::Read::read_exact(&mut cursor, &mut len_buf).is_err() {
                break;
            }

            let len_hex =
                from_utf8(&len_buf).map_err(|e| MegaError::with_message(format!("{e}")))?;
            let len = u32::from_str_radix(len_hex, 16)
                .map_err(|e| MegaError::with_message(format!("{e}")))?;

            if len == 0 {
                continue;
            }

            let mut data = vec![0u8; (len - 4) as usize];
            std::io::Read::read_exact(&mut cursor, &mut data)?;
            let line = String::from_utf8_lossy(&data);

            if line.contains("refs/heads/") || line.contains("refs/tags/") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "refs/heads/main" {
                    refs.push(parts[0].to_string());
                    break;
                }
            }
        }

        Ok(refs)
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
            .map_err(|e| MegaError::with_message(format!("Failed to send request: {}", e)))?;

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
