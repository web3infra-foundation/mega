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

use crate::protocol::{pack, PackProtocol};

/// # Build Response headers for Smart Server.
/// Clients MUST NOT reuse or revalidate a cached response.
/// Servers MUST include sufficient Cache-Control headers to prevent caching of the response.
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

/// # Sends a Git pack to the remote server.
///
/// This function takes a `Sender` for sending data to the remote server, the `result` vector
/// containing the pack data, and the `pack_protocol` describing the pack transfer protocol.
/// It asynchronously reads the pack data from the `result` vector in chunks, formats it using the
/// side-band format specified by the `pack_protocol`, and sends it to the remote server using
/// the `Sender`.
///
/// # Arguments
///
/// * `sender` - The sender for sending data to the remote server.
/// * `result` - The vector containing the pack data to be sent.
/// * `pack_protocol` - The pack protocol describing the pack transfer.
///
/// # Returns
///
/// * `Ok(())` - If the pack data is successfully sent to the remote server.
/// * `Err((StatusCode, &'static str))` - If there is an error during the sending process, with the
///   error status code and a corresponding error message.
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
        tracing::info!("send: packet length: {:?}", bytes_out.len());
        sender.send_data(bytes_out.freeze()).await.unwrap();
    }
}
/// # Handles a Git upload pack request and prepares the response.
///
/// The function takes a `req` parameter representing the HTTP request received and a `pack_protocol`
/// parameter containing the configuration for the Git pack protocol.
///
/// The function extracts the request body into a `BytesMut` buffer by iterating over the chunks
/// of the request body using `body.next().await`. The chunks are concatenated into the `upload_request`
/// buffer.
///
/// The `pack_protocol` is then used to process the `upload_request` using the `git_upload_pack` method.
/// It returns the `send_pack_data` and `buf` containing the response data.
///
/// A response header is constructed using the `build_res_header` function with a content type of
/// "application/x-git-upload-pack-result". The response body channel is created using `Body::channel()`.
///
/// The `buf` is sent as the initial data using the `sender` to establish the response body.
///
/// A new task is spawned to send the remaining `send_pack_data` using the `send_pack` function.
///
/// Finally, the constructed response with the response body is returned.
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

/// # Handles a Git receive pack request and prepares the response.
///
/// The function takes a `req` parameter representing the HTTP request received and a `pack_protocol`
/// parameter containing the configuration for the Git pack protocol.
///
/// The function extracts the request body into a vector of bytes, `combined_body_bytes`, by iterating over the
/// chunks of the request body using `body.next().await`. The chunks are appended to the `combined_body_bytes`.
///
/// The `pack_protocol` is then used to process the `combined_body_bytes` using the `git_receive_pack` method.
/// It returns the `pack_data` containing the response data.
///
/// The `pack_data` is passed to the `git_receive_pack` method again to obtain the final response data as a `buf`.
///
/// The `buf` is converted into a `Body` using `Body::from()` and assigned to `body`.
/// Tracing information is logged regarding the status of the response body.
///
/// A response header is constructed using the `build_res_header` function with a content type of
/// "application/x-git-receive-pack-result". The response body is set to `body`.
///
/// Finally, the constructed response is returned.
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

    let parse_report = pack_protocol
        .git_receive_pack(Bytes::from(combined_body_bytes))
        .await
        .unwrap();
    let body = Body::from(parse_report);
    tracing::info!("report status:{:?}", body);
    let resp = build_res_header("application/x-git-receive-pack-result".to_owned());

    let resp = resp.body(body).unwrap();
    Ok(resp)
}
#[cfg(test)]
mod tests {}
