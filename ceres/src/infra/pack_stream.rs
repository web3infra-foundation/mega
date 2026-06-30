use std::pin::Pin;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};

/// Framework-neutral receive-pack body stream (decoupled from axum).
pub type PackStreamError = Box<dyn std::error::Error + Send + Sync>;
pub type PackByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, PackStreamError>> + Send>>;

pub fn into_pack_byte_stream<S, E>(stream: S) -> PackByteStream
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    Box::pin(stream.map_err(|e| Box::new(e) as PackStreamError))
}
