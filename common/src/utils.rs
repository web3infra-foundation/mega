//!
//!
//!
use std::io::Read;
use std::io::Result;

pub const ZERO_ID: &str = match std::str::from_utf8(&[b'0'; 40]) {
    Ok(s) => s,
    Err(_) => panic!("can't get ZERO_ID"),
};

/// Read the next N bytes from the reader
///
#[inline]
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;

    Ok(bytes)
}

/// Read a u32 from the reader
///
pub fn read_u32<R: Read>(stream: &mut R) -> Result<u32> {
    let bytes = read_bytes(stream)?;

    Ok(u32::from_be_bytes(bytes))
}