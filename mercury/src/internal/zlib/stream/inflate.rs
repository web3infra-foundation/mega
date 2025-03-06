use std::{io, io::BufRead};

use flate2::{Decompress, FlushDecompress, Status};
use sha1::{digest::core_api::CoreWrapper, Digest, Sha1};

use crate::internal::object::types::ObjectType;

/// ReadBoxed is to unzip information from a  DEFLATE stream,
/// which hash [`BufRead`] trait.
/// For a continuous stream of DEFLATE information, the structure
/// does not read too many bytes to affect subsequent information
/// reads
pub struct ReadBoxed<R> {
    /// The reader from which bytes should be decompressed.
    pub inner: R,
    /// The decompressor doing all the work.
    pub decompressor: Box<Decompress>,
    /// the [`count_hash`] decide whether to calculate the hash value in the [`read`] method
    count_hash: bool,
    pub hash: CoreWrapper<sha1::Sha1Core>,
}
impl<R> ReadBoxed<R>
where
    R: BufRead,
{
    /// Nen a ReadBoxed for zlib read, the Output ReadBoxed is for the Common Object,
    /// but not for the Delta Object,if that ,see new_for_delta method below.
    pub fn new(inner: R, obj_type: ObjectType, size: usize) -> Self {
        let mut hash = sha1::Sha1::new();
        hash.update(obj_type.to_bytes());
        hash.update(b" ");
        hash.update(size.to_string());
        hash.update(b"\0");
        ReadBoxed {
            inner,
            hash,
            count_hash: true,
            decompressor: Box::new(Decompress::new(true)),
        }
    }

    pub fn new_for_delta(inner: R) -> Self {
        ReadBoxed {
            inner,
            hash: Sha1::new(),
            count_hash: false,
            decompressor: Box::new(Decompress::new(true)),
        }
    }
}
impl<R> io::Read for ReadBoxed<R>
where
    R: BufRead,
{
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let o = read(&mut self.inner, &mut self.decompressor, into)?;
        //update the hash value
        if self.count_hash {
            self.hash.update(&into[..o]);
        }
        Ok(o)
    }
}

/// Read bytes from `rd` and decompress them using `state` into a pre-allocated fitting buffer `dst`, returning the amount of bytes written.
fn read(rd: &mut impl BufRead, state: &mut Decompress, mut dst: &mut [u8]) -> io::Result<usize> {
    let mut total_written = 0;
    loop {
        let (written, consumed, ret, eof);
        {
            let input = rd.fill_buf()?;
            eof = input.is_empty();
            let before_out = state.total_out();
            let before_in = state.total_in();
            let flush = if eof {
                FlushDecompress::Finish
            } else {
                FlushDecompress::None
            };
            ret = state.decompress(input, dst, flush);
            written = (state.total_out() - before_out) as usize;
            total_written += written;
            dst = &mut dst[written..];
            consumed = (state.total_in() - before_in) as usize;
        }
        rd.consume(consumed);

        match ret {
            // The stream has officially ended, nothing more to do here.
            Ok(Status::StreamEnd) => return Ok(total_written),
            // Either input our output are depleted even though the stream is not depleted yet.
            Ok(Status::Ok | Status::BufError) if eof || dst.is_empty() => return Ok(total_written),
            // Some progress was made in both the input and the output, it must continue to reach the end.
            Ok(Status::Ok | Status::BufError) if consumed != 0 || written != 0 => continue,
            // A strange state, where zlib makes no progress but isn't done either. Call it out.
            Ok(Status::Ok | Status::BufError) => unreachable!("Definitely a bug somewhere"),
            Err(..) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "corrupt deflate stream",
                ))
            }
        }
    }
}
