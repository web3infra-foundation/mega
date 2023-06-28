use crate::internal::ObjectType;
use flate2::read::ZlibDecoder;
use sha1::{Digest, Sha1};
use std::io::{self, Read, Seek};

pub struct ObjReader<'a, R> {
    inner: ZlibDecoder<&'a mut R>,
    sha1: Sha1,
    _len: usize,
}

impl<'a, R: Read + Seek + Send> ObjReader<'a, R> {
    pub fn new(inner: &'a mut R, len: usize, obj_type: ObjectType) -> Self {
        let mut h = Sha1::new();
        h.update(obj_type.to_bytes());
        h.update(b" ");
        h.update(len.to_string());
        h.update(b"\0");
        Self {
            inner: ZlibDecoder::new(inner),
            sha1: h,
            _len: len,
        }
    }
    pub fn finalize(self) -> String {
        let hash = self.sha1.finalize();
        format!("{:x}", hash)
    }
    pub fn glen(&self) -> usize {
        self._len
    }
}

impl<R: Read + Seek + Send> Read for ObjReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.inner.read(buf)?;
        self.sha1.update(&buf[..bytes_read]);
        Ok(bytes_read)
    }
}
