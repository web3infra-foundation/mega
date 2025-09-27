use bytes::{Bytes, BytesMut};
use ceres::protocol::ServiceType;
use ceres::protocol::smart::{add_pkt_line_string, read_pkt_line};
use mercury::errors::GitError;
use mercury::hash::SHA1;
use url::Url;

pub mod https_client;
pub mod lfs_client;
pub mod local_client;

#[allow(dead_code)] // todo: unimplemented
pub trait ProtocolClient {
    /// create client from url
    fn from_url(url: &Url) -> Self;
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredReference {
    pub(crate) _hash: String,
    pub(crate) _ref: String,
}

pub type DiscRef = DiscoveredReference;

pub type FetchStream = futures_util::stream::BoxStream<'static, Result<Bytes, std::io::Error>>;

pub fn parse_discovered_references(
    mut response_content: Bytes,
    service: ServiceType,
) -> Result<Vec<DiscRef>, GitError> {
    let mut ref_list = vec![];
    let mut saw_header = false;
    let mut processed_first_ref = false;

    loop {
        let (bytes_take, pkt_line) = read_pkt_line(&mut response_content);
        if bytes_take == 0 {
            if response_content.is_empty() {
                break;
            } else {
                continue;
            }
        }

        if !saw_header && pkt_line.starts_with(b"# service=") {
            let header = String::from_utf8(pkt_line.to_vec()).map_err(|e| {
                GitError::NetworkError(format!("Invalid UTF-8 in response header: {}", e))
            })?;
            tracing::debug!("discovery header: {header:?}");
            saw_header = true;
            continue;
        }
        saw_header = true;

        let pkt_line = String::from_utf8(pkt_line.to_vec())
            .map_err(|e| GitError::NetworkError(format!("Invalid UTF-8 in response: {}", e)))?;
        if pkt_line.len() < 40 {
            return Err(GitError::NetworkError(
                "Invalid reference format, missing object id".to_string(),
            ));
        }
        let (hash, rest) = pkt_line.split_at(40);
        let hash = hash.to_string();
        let rest = rest.trim();

        if !processed_first_ref {
            if hash == SHA1::default().to_string() {
                tracing::debug!(
                    "discovery for {:?} returned zero hash, treating as empty repository",
                    service
                );
                break;
            }
            let (reference, caps) = match rest.split_once('\0') {
                Some((r, c)) => (r, c),
                None => (rest, ""),
            };
            if reference != "capabilities^{}" {
                ref_list.push(DiscoveredReference {
                    _hash: hash.clone(),
                    _ref: reference.to_string(),
                });
            }
            if !caps.is_empty() {
                let caps = caps.split(' ').collect::<Vec<&str>>();
                tracing::debug!("capability declarations: {:?}", caps);
            }
            processed_first_ref = true;
        } else {
            ref_list.push(DiscoveredReference {
                _hash: hash,
                _ref: rest.to_string(),
            });
        }
    }
    Ok(ref_list)
}

pub fn generate_upload_pack_content(have: &[String], want: &[String]) -> Bytes {
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
    buf.extend(b"0000");
    for h in have {
        add_pkt_line_string(&mut buf, format!("have {h}\n").to_string());
    }

    add_pkt_line_string(&mut buf, "done\n".to_string());

    buf.freeze()
}

#[cfg(test)]
mod test {}
