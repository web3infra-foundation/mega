use sha1::digest::core_api::CoreWrapper;
use sha1::{Digest, Sha1};
use std::io::{BufRead, BufReader, Cursor, ErrorKind, Read};
use std::sync::Arc;

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
    result: BufReader<Cursor<Vec<u8>>>,
    len: usize,
    pub hash: CoreWrapper<sha1::Sha1Core>,
}
impl DeltaReader {
    pub async fn new(reader: &mut impl Read, base_object: Arc<dyn ObjectT>) -> Self {
        let copy_obj = base_object.clone();
        let buffer = AsyncDeltaBuffer::new(reader, base_object).await;

        let mut h = Sha1::new();
        h.update(copy_obj.get_type().to_bytes());
        h.update(b" ");
        h.update(buffer.result_size.to_string());
        h.update(b"\0");

        //buffer.read_to_end(&mut result).await.unwrap();
        let data = buffer.inner;
        let result: Vec<u8> = data.clone();
        drop(data);

        Self {
            len: result.len(),
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
    inner: Vec<u8>,
    result_size: usize,
}

impl AsyncDeltaBuffer {
    async fn new(mut stream: &mut impl Read, base_object: Arc<dyn ObjectT>) -> Self {
        // Read the bash object size & Result Size
        let base_size = utils::read_size_encoding(&mut stream).unwrap();
        let result_size = utils::read_size_encoding(&mut stream).unwrap();

        //Get the base object row data
        let base_info: &[u8] = &base_object.get_raw();
        assert_eq!(base_info.len(), base_size);

        let mut inner = Vec::with_capacity(result_size);

        process_delta(&mut stream, &mut inner, base_object).await;

        AsyncDeltaBuffer { inner, result_size }
    }
}
/// Compte the Delta Object based on the "base object"
///
async fn process_delta(
    mut stream: &mut impl Read,
    buffer: &mut Vec<u8>,
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
            buffer.extend_from_slice(&data);
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
                Ok(data) => buffer.extend_from_slice(data),
                Err(e) => panic!("{}", e),
            }
        }
    }
}

pub fn undelta(mut stream: &mut impl Read, base_info: &Vec<u8>) -> Result<Vec<u8>,GitError> {
    // Read the bash object size & Result Size
    let base_size = utils::read_size_encoding(&mut stream).unwrap();
    if base_info.len() != base_size{
        return Err(GitError::DeltaObjectError("base object len is not equal".to_owned()));
    }
    

    let result_size = utils::read_size_encoding(&mut stream).unwrap();
    let mut buffer = Vec::with_capacity(result_size);
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
            buffer.extend_from_slice(&data);
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
                Ok(data) => buffer.extend_from_slice(data),
                Err(e) => panic!("{}", e),
            }
        }
    }
    assert!(buffer.len() == result_size);
    Ok(buffer)
}
#[cfg(test)]
mod tests {}
