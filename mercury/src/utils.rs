use crate::hash::SHA1;
use std::io;
use std::io::{BufRead, Read};

pub fn read_bytes(file: &mut impl Read, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_sha1(file: &mut impl Read) -> io::Result<SHA1> {
    SHA1::from_stream(file)
}




/// A lightweight wrapper that counts bytes read from the underlying reader.
/// replace deflate.intotal() in decompress_data
pub struct CountingReader<R> {
    pub inner: R,
    pub bytes_read: u64,
}

impl<R> CountingReader<R> {
    /// Creates a new `CountingReader` wrapping the given reader.
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            bytes_read: 0,
        }
    }
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.bytes_read += n as u64;
        Ok(n)
    }
}

impl<R: BufRead> BufRead for CountingReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.bytes_read += amt as u64; 
        self.inner.consume(amt);
    }
}
