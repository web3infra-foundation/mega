//!
//!
//!
//!
//!
use std::collections::HashMap;

use anyhow::Result;
use axum::body::Body;
use axum::http::response::Builder;
use axum::http::{Request, Response, StatusCode};
use bytes::{Bytes, BytesMut};
use futures::TryStreamExt;
use tokio::io::AsyncReadExt;

use git::protocol::{pack, PackProtocol, ServiceType};

use crate::https::GetParams;

// # Discovering Reference
// HTTP clients that support the "smart" protocol (or both the "smart" and "dumb" protocols) MUST
// discover references by making a parameterized request for the info/refs file of the repository.
// The request MUST contain exactly one query parameter, service=$servicename,
// where $servicename MUST be the service name the client wishes to contact to complete the operation.
// The request MUST NOT contain additional query parameters.
pub async fn git_info_refs(
    params: GetParams,
    mut pack_protocol: PackProtocol,
) -> Result<Response<Body>, (StatusCode, String)> {
    let service_name = params.service.unwrap();
    let service_type = service_name.parse::<ServiceType>().unwrap();
    let resp = build_res_header(format!("application/x-{}-advertisement", service_name));
    let pkt_line_stream = pack_protocol.git_info_refs(service_type).await;
    let body = Body::from(pkt_line_stream.freeze());
    Ok(resp.body(body).unwrap())
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
    let upload_request: BytesMut = req
        .into_body()
        .into_data_stream()
        .try_fold(BytesMut::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .unwrap();

    let (send_pack_data, buf) = pack_protocol
        .git_upload_pack(&mut upload_request.freeze())
        .await
        .unwrap();
    tracing::info!("send ack/nak message buf: {:?}", buf);
    let mut res_bytes = BytesMut::new();
    res_bytes.extend(buf);

    let resp = build_res_header("application/x-git-upload-pack-result".to_owned());

    tracing::info!("send response");

    let mut reader = send_pack_data.as_slice();
    loop {
        let mut temp = BytesMut::new();
        temp.reserve(65500);
        let length = reader.read_buf(&mut temp).await.unwrap();
        if length == 0 {
            let bytes_out = Bytes::from_static(pack::PKT_LINE_END_MARKER);
            tracing::info!("send 0000:{:?}", bytes_out);
            res_bytes.extend(bytes_out);
            break;
        }
        let bytes_out = pack_protocol.build_side_band_format(temp, length);
        tracing::info!("send pack file: length: {:?}", bytes_out.len());
        res_bytes.extend(bytes_out);
    }
    let body = Body::from(res_bytes.freeze());
    let resp = resp.body(body).unwrap();
    Ok(resp)
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
    let combined_body_bytes: BytesMut = req
        .into_body()
        .into_data_stream()
        .try_fold(BytesMut::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .unwrap();


    let parse_report = pack_protocol
        .git_receive_pack(combined_body_bytes.freeze())
        .await
        .unwrap();
    tracing::info!("report status:{:?}", parse_report);
    let resp = build_res_header("application/x-git-receive-pack-result".to_owned());
    let resp = resp.body(Body::from(parse_report)).unwrap();
    Ok(resp)
}

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
#[cfg(test)]
mod tests {}
