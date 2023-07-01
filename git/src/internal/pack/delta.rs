use std::io::{BufRead, BufReader, Cursor, ErrorKind, Read,Error};
use std::sync::Arc;
use sha1::digest::core_api::CoreWrapper;
use sha1::{Digest, Sha1};

use tokio::io::AsyncRead;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::internal::ObjectType;
use crate::internal::object::ObjectT;
use crate::{errors::GitError, utils};

const COPY_INSTRUCTION_FLAG: u8 = 1 << 7;
const COPY_OFFSET_BYTES: u8 = 4;
const COPY_SIZE_BYTES: u8 = 3;
const COPY_ZERO_SIZE: usize = 0x10000;

/// The Delta Reader to deal with the Delta Object. 
/// 
/// Impl The [`Read`] trait and [`BufRead`] trait.
/// Receive a Read object, decompress the data in it with zlib, and 
/// return it to Object after delta processing.
pub struct DeltaReader {
    #[allow(unused)]
    inner: AsyncDeltaBuffer,
    result: BufReader<Cursor<Vec<u8>>>,
    len: usize,
    pub hash: CoreWrapper<sha1::Sha1Core>,
}
impl DeltaReader {
    pub async fn new(reader: &mut impl Read, bash_object: Arc<dyn ObjectT>) -> Self {
        let copy_obj = bash_object.clone();
        let buffer = AsyncDeltaBuffer::new(reader, bash_object).await;

        let mut h = Sha1::new();
        h.update(copy_obj.get_type().to_bytes());
        h.update(b" ");
        h.update(buffer.result_size.to_string());
        h.update(b"\0");

        //buffer.read_to_end(&mut result).await.unwrap();
        let data = buffer.inner.lock().await;
        let result: Vec<u8> = data.clone();
        drop(data);

        Self {
            len: result.len(),
            inner: buffer,
            result: BufReader::with_capacity(4096, Cursor::new(result)),
            hash: h,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
impl Read for DeltaReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let o = self.result.read(buf)?;
        //
        self.hash.update(&buf[..o]);
        Ok(o)
    }
}

impl BufRead for DeltaReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.result.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.result.consume(amt);
    }
}

struct AsyncDeltaBuffer {
    inner: Arc<Mutex<Vec<u8>>>,
    #[allow(unused)]
    obj_type: ObjectType,
    #[allow(unused)]
    processing_task: Option<JoinHandle<()>>,
    result_size: usize,
}

impl AsyncDeltaBuffer {
    async fn new(mut stream: &mut impl Read, bash_object: Arc<dyn ObjectT>) -> Self {
        // Read the bash object size & Result Size
        let base_size = utils::read_size_encoding(&mut stream).unwrap();
        let result_size = utils::read_size_encoding(&mut stream).unwrap();

        //Get the base object row data
        let base_info: &[u8] = bash_object.get_raw();
        assert_eq!(base_info.len(), base_size);

        let obj_type = bash_object.get_type();

        let inner: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::with_capacity(result_size)));
        let in_arc = Arc::clone(&inner);

        // let processing_task = tokio::spawn( async move  {
        //     process_delta(&mut stream,in_arc,bash_object);
        // });
        process_delta(&mut stream, in_arc, bash_object).await;

        AsyncDeltaBuffer {
            inner,
            obj_type,
            processing_task: None,
            result_size,
        }
    }

}

impl AsyncRead for AsyncDeltaBuffer {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<Result<(), Error>> {
        let inner = self.inner.try_lock();
        if let Ok(mut guard) = inner {
            if guard.is_empty() {
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            } else {
                let bytes_read = std::cmp::min(buf.remaining(), guard.len());
                buf.put_slice(&guard[..bytes_read]);
                guard.drain(..bytes_read);
                std::task::Poll::Ready(Ok(()))
            }
        } else {
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }
}

async fn process_delta(
    mut stream: &mut impl Read,
    buffer: Arc<Mutex<Vec<u8>>>,
    base_object: Arc<dyn ObjectT>,
) {
    let base_info = base_object.get_raw();
    loop {
        // Check if the stream has ended, meaning the new object is done
        let instruction = match utils::read_bytes(stream) {
            Ok([instruction]) => instruction,
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(err) => {
                panic!(
                    "{}",
                    GitError::DeltaObjectError(format!("Wrong instruction in delta :{}", err))
                );
            }
        };

        if instruction & COPY_INSTRUCTION_FLAG == 0 {
            // Data instruction; the instruction byte specifies the number of data bytes
            if instruction == 0 {
                // Appending 0 bytes doesn't make sense, so git disallows it
                panic!(
                    "{}",
                    GitError::DeltaObjectError(String::from("Invalid data instruction"))
                );
            }

            // Append the provided bytes
            let mut data = vec![0; instruction as usize];
            stream.read_exact(&mut data).unwrap();
            buffer.lock().await.extend_from_slice(&data);
        // result.extend_from_slice(&data);
        } else {
            // Copy instruction
            let mut nonzero_bytes = instruction;
            let offset =
                utils::read_partial_int(&mut stream, COPY_OFFSET_BYTES, &mut nonzero_bytes)
                    .unwrap();
            let mut size =
                utils::read_partial_int(&mut stream, COPY_SIZE_BYTES, &mut nonzero_bytes).unwrap();
            if size == 0 {
                // Copying 0 bytes doesn't make sense, so git assumes a different size
                size = COPY_ZERO_SIZE;
            }
            // Copy bytes from the base object
            let base_data = base_info
                .get(offset..(offset + size))
                .ok_or_else(|| GitError::DeltaObjectError("Invalid copy instruction".to_string()));

            match base_data {
                Ok(data) => buffer.lock().await.extend_from_slice(data),
                Err(e) => panic!("{}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {}
