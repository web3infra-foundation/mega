use bytes::Bytes;
use git_internal::internal::object::blob::Blob;

use crate::object_storage::ObjectByteStream;

pub trait IntoObjectStream {
    fn into_stream(self) -> ObjectByteStream;
}

impl IntoObjectStream for Blob {
    fn into_stream(self) -> ObjectByteStream {
        Box::pin(futures::stream::once(
            async move { Ok(Bytes::from(self.data)) },
        ))
    }
}

impl IntoObjectStream for Vec<u8> {
    fn into_stream(self) -> ObjectByteStream {
        Box::pin(futures::stream::once(async move { Ok(Bytes::from(self)) }))
    }
}
impl IntoObjectStream for Bytes {
    fn into_stream(self) -> ObjectByteStream {
        Box::pin(futures::stream::once(async move { Ok(self) }))
    }
}
