//!
//!
//!
//!
//!
use std::collections::HashMap;

use anyhow::Result;
use axum::body::Body;
use axum::http::response::Builder;
use axum::http::{Response, StatusCode};

use bytes::{BufMut, Bytes, BytesMut};

use futures::StreamExt;
use hyper::body::Sender;
use hyper::Request;

use tokio::io::{AsyncReadExt, BufReader};

use super::{pack, PackProtocol};

pub fn build_res_header(content_type: String) -> Builder {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), content_type);
    headers.insert(
        "Cache-Control".to_string(),
        "no-cache, max-age=0, must-revalidate".to_string(),
    );
    let mut resp = Response::builder();

    for (key, val) in headers {
        resp = resp.header(&key, val);
    }
    resp
}

pub async fn send_pack(
    mut sender: Sender,
    result: Vec<u8>,
    pack_protocol: PackProtocol,
) -> Result<(), (StatusCode, &'static str)> {
    let mut reader = BufReader::new(result.as_slice());
    loop {
        let mut temp = BytesMut::new();
        temp.reserve(65500);
        let length = reader.read_buf(&mut temp).await.unwrap();
        if temp.is_empty() {
            let mut bytes_out = BytesMut::new();
            bytes_out.put_slice(pack::PKT_LINE_END_MARKER);
            tracing::info!("send: bytes_out: {:?}", bytes_out.clone().freeze());
            sender.send_data(bytes_out.freeze()).await.unwrap();
            return Ok(());
        }
        let bytes_out = pack_protocol.build_side_band_format(temp, length);
        tracing::info!("send: bytes_out: {:?}", bytes_out.clone().freeze());
        sender.send_data(bytes_out.freeze()).await.unwrap();
    }
}

pub async fn git_upload_pack(
    req: Request<Body>,
    mut pack_protocol: PackProtocol,
) -> Result<Response<Body>, (StatusCode, String)> {
    let (_parts, mut body) = req.into_parts();

    let mut upload_request = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        upload_request.extend_from_slice(&bytes);
    }

    let (send_pack_data, buf) = pack_protocol
        .git_upload_pack(&mut upload_request.freeze())
        .await
        .unwrap();
    let resp = build_res_header("application/x-git-upload-pack-result".to_owned());

    tracing::info!("send buf: {:?}", buf);

    let (mut sender, body) = Body::channel();
    sender.send_data(buf.freeze()).await.unwrap();

    tokio::spawn(send_pack(sender, send_pack_data, pack_protocol));
    Ok(resp.body(body).unwrap())
}

pub async fn git_receive_pack(
    req: Request<Body>,
    mut pack_protocol: PackProtocol,
) -> Result<Response<Body>, (StatusCode, String)> {
    let (_parts, mut body) = req.into_parts();
    let mut combined_body_bytes = Vec::new();
    while let Some(chunk) = body.next().await {
        let body_bytes = chunk.unwrap();
        combined_body_bytes.extend(&body_bytes);
    }

    let pack_data = pack_protocol
        .git_receive_pack(Bytes::from(combined_body_bytes))
        .await
        .unwrap();

    let buf = pack_protocol.git_receive_pack(pack_data).await.unwrap();

    let body = Body::from(buf);
    tracing::info!("report status:{:?}", body);
    let resp = build_res_header("application/x-git-receive-pack-result".to_owned());

    let resp = resp.body(body).unwrap();
    Ok(resp)
}
#[cfg(test)]
mod tests {}
