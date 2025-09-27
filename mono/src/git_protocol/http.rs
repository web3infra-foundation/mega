use std::convert::Infallible;

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderValue, Request, Response};
use bytes::{Bytes, BytesMut};
use futures::{TryStreamExt, stream};
use tokio::io::AsyncReadExt;
use tokio_stream::StreamExt;

use ceres::protocol::{ServiceType, SmartProtocol, smart};
use common::errors::ProtocolError;
use common::model::InfoRefsParams;

// # Discovering Reference
// HTTP clients that support the "smart" protocol (or both the "smart" and "dumb" protocols) MUST
// discover references by making a parameterized request for the info/refs file of the repository.
// The request MUST contain exactly one query parameter, service=$servicename,
// where $servicename MUST be the service name the client wishes to contact to complete the operation.
// The request MUST NOT contain additional query parameters.
pub async fn git_info_refs(
    params: InfoRefsParams,
    mut pack_protocol: SmartProtocol,
) -> Result<Response<Body>, ProtocolError> {
    let service_name = params.service.unwrap();
    pack_protocol.service_type = Some(service_name.parse::<ServiceType>().unwrap());

    let pkt_line_stream = pack_protocol.git_info_refs().await?;

    let content_type = format!("application/x-{service_name}-advertisement");
    let response = add_default_header(
        content_type,
        Response::builder()
            .body(Body::from(pkt_line_stream.freeze()))
            .unwrap(),
    );
    Ok(response)
}

fn auth_failed() -> Result<Response<Body>, ProtocolError> {
    let resp = Response::builder()
        .status(401)
        .header(
            http::header::WWW_AUTHENTICATE,
            HeaderValue::from_static("Basic realm=Mega"),
        )
        .body(Body::empty())
        .unwrap();
    Ok(resp)
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
    mut pack_protocol: SmartProtocol,
) -> Result<Response<Body>, ProtocolError> {
    let upload_request: BytesMut = req
        .into_body()
        .into_data_stream()
        .try_fold(BytesMut::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .unwrap();
    tracing::debug!("Receive bytes: <-------- {:?}", upload_request);
    let (mut send_pack_data, protocol_buf) = pack_protocol
        .git_upload_pack(&mut upload_request.freeze())
        .await?;

    let body_stream = async_stream::stream! {
        tracing::info!("send ack/nak message buf: --------> {:?}", &protocol_buf);
        yield Ok::<_, Infallible>(Bytes::copy_from_slice(&protocol_buf));
        // send packdata with sideband64k
        while let Some(chunk) = send_pack_data.next().await {
            let mut reader = chunk.as_slice();
            loop {
                let mut temp = BytesMut::new();
                temp.reserve(65500);
                let length = reader.read_buf(&mut temp).await.unwrap();
                if length == 0 {
                    break;
                }
                let bytes_out = pack_protocol.build_side_band_format(temp, length);
                // tracing::info!("send pack file: length: {:?}", bytes_out.len());
                yield Ok::<_, Infallible>(bytes_out.freeze());
            }
        }
        let bytes_out = Bytes::from_static(smart::PKT_LINE_END_MARKER);
        tracing::info!("send back pkt-flush line '0000', actually: {:?}", bytes_out);
        yield Ok::<_, Infallible>(bytes_out);
    };
    let response = add_default_header(
        String::from("application/x-git-upload-pack-result"),
        Response::builder()
            .body(Body::from_stream(body_stream))
            .unwrap(),
    );
    Ok(response)
}

/// Handles the Git receive-pack protocol for receiving and processing data from a client.
///
/// This asynchronous function processes an HTTP request to handle the Git "receive-pack" service,
/// which is used for receiving data when pushing changes to a Git repository. The function reads
/// data from the request body, processes it according to the Git smart protocol, and sends back
/// a response indicating the status of the operation.
///
/// # Parameters
/// - `req`: The incoming HTTP request containing the body stream with the Git data.
/// - `pack_protocol`: A mutable instance of `SmartProtocol` used to process the Git receive-pack protocol.
///
/// # Returns
/// A `Result` containing either:
/// - `Response<Body>`: The HTTP response with the result of the receive-pack operation.
/// - `(StatusCode, String)`: A tuple with an HTTP status code and an error message in case of failure.
pub async fn git_receive_pack(
    req: Request<Body>,
    mut pack_protocol: SmartProtocol,
) -> Result<Response<Body>, ProtocolError> {
    if pack_protocol.enable_http_auth() && !pack_protocol.http_auth(req.headers()).await {
        return auth_failed();
    }
    // Convert the request body into a data stream.
    let mut data_stream = req.into_body().into_data_stream();
    let mut report_status = Bytes::new();

    let mut chunk_buffer = BytesMut::new(); // Used to cache the data of chunks before the PACK subsequence is found.
    // Process the data stream to handle the Git receive-pack protocol.
    while let Some(chunk) = data_stream.next().await {
        let chunk = chunk.unwrap();
        // Process the data up to the "PACK" subsequence.
        if let Some(pos) = search_subsequence(&chunk, b"PACK") {
            chunk_buffer.extend_from_slice(&chunk[0..pos]);
            pack_protocol.parse_receive_pack_commands(Bytes::copy_from_slice(&chunk_buffer));
            // Create a new stream from the remaining bytes and the rest of the data stream.
            let left_chunk_bytes = Bytes::copy_from_slice(&chunk[pos..]);
            let pack_stream = stream::once(async { Ok(left_chunk_bytes) }).chain(data_stream);
            report_status = pack_protocol
                .git_receive_pack_stream(Box::pin(pack_stream))
                .await?;
            break;
        } else {
            chunk_buffer.extend_from_slice(&chunk);
        }
    }
    tracing::info!("report status:{:?}", report_status);
    let response = Response::builder().body(Body::from(report_status)).unwrap();
    let response = add_default_header(
        String::from("application/x-git-receive-pack-result"),
        response,
    );
    Ok(response)
}

// Function to find the subsequence in a slice
pub fn search_subsequence(chunk: &[u8], search: &[u8]) -> Option<usize> {
    chunk.windows(search.len()).position(|s| s == search)
}

/// # Build Response headers for Smart Server.
/// Clients MUST NOT reuse or revalidate a cached response.
/// Servers MUST include sufficient Cache-Control headers to prevent caching of the response.
fn add_default_header<T>(content_type: String, mut response: Response<T>) -> Response<T> {
    response.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_str(&content_type).unwrap(),
    );
    response.headers_mut().insert(
        "Cache-Control",
        HeaderValue::from_static("no-cache, max-age=0, must-revalidate"),
    );
    response
}

#[cfg(test)]
mod tests {}
