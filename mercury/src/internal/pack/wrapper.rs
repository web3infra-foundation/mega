use std::io::{self, BufRead, Read};

use sha1::{Digest, Sha1};

use crate::hash::SHA1;

/// [`Wrapper`] is a wrapper around a reader that also computes the SHA1 hash of the data read.
///
/// It is designed to work with any reader that implements `BufRead`.
///
/// Fields:
/// * `inner`: The inner reader.
/// * `hash`: The SHA1 hash state.
/// * `count_hash`: A flag to indicate whether to compute the hash while reading.
pub struct Wrapper<R> {
    inner: R,
    hash: Sha1,
    bytes_read: usize,
}

impl<R> Wrapper<R>
where
    R: BufRead,
{
    /// Constructs a new [`Wrapper`] with the given reader and a flag to enable or disable hashing.
    ///
    /// # Parameters
    /// * `inner`: The reader to wrap.
    /// * `count_hash`: If `true`, the hash is computed while reading; otherwise, it is not.
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            hash: Sha1::new(), // Initialize a new SHA1 hasher
            bytes_read: 0,
        }
    }

    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }

    /// Returns the final SHA1 hash of the data read so far.
    ///
    /// This is a clone of the internal hash state finalized into a SHA1 hash.
    pub fn final_hash(&self) -> SHA1 {
        let re: [u8; 20] = self.hash.clone().finalize().into(); // Clone, finalize, and convert the hash into bytes
        SHA1(re)
    }
}

impl<R> BufRead for Wrapper<R>
where
    R: BufRead,
{
    /// Provides access to the internal buffer of the wrapped reader without consuming it.
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf() // Delegate to the inner reader
    }

    /// Consumes data from the buffer and updates the hash if `count_hash` is true.
    ///
    /// # Parameters
    /// * `amt`: The amount of data to consume from the buffer.
    fn consume(&mut self, amt: usize) {
        let buffer = self.inner.fill_buf().expect("Failed to fill buffer");
        self.hash.update(&buffer[..amt]); // Update hash with the data being consumed
        self.inner.consume(amt); // Consume the data from the inner reader
        self.bytes_read += amt;
    }
}

impl<R> Read for Wrapper<R>
where
    R: BufRead,
{
    /// Reads data into the provided buffer and updates the hash if `count_hash` is true.
    /// <br> [Read::read_exact] calls it internally.
    ///
    /// # Parameters
    /// * `buf`: The buffer to read data into.
    ///
    /// # Returns
    /// Returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let o = self.inner.read(buf)?; // Read data into the buffer
        self.hash.update(&buf[..o]); // Update hash with the data being read
        self.bytes_read += o;
        Ok(o) // Return the number of bytes read
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, BufReader, Cursor, Read};

    use sha1::{Digest, Sha1};

    use crate::internal::pack::wrapper::Wrapper;

    #[test]
    fn test_wrapper_read() -> io::Result<()> {
        let data = b"Hello, world!"; // Sample data
        let cursor = Cursor::new(data.as_ref());
        let buf_reader = BufReader::new(cursor);
        let mut wrapper = Wrapper::new(buf_reader);

        let mut buffer = vec![0; data.len()];
        wrapper.read_exact(&mut buffer)?;

        assert_eq!(buffer, data);
        Ok(())
    }

    #[test]
    fn test_wrapper_hash() -> io::Result<()> {
        let data = b"Hello, world!";
        let cursor = Cursor::new(data.as_ref());
        let buf_reader = BufReader::new(cursor);
        let mut wrapper = Wrapper::new(buf_reader);

        let mut buffer = vec![0; data.len()];
        wrapper.read_exact(&mut buffer)?;

        let hash_result = wrapper.final_hash();
        let mut hasher = Sha1::new();
        hasher.update(data);
        let expected_hash: [u8; 20] = hasher.finalize().into();

        assert_eq!(hash_result.0, expected_hash);
        Ok(())
    }
}
