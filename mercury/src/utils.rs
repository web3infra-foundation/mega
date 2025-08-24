#![allow(dead_code)]

use std::io;
use std::io::Read;

use crate::hash::SHA1;

pub fn read_bytes(file: &mut impl Read, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_sha1(file: &mut impl Read) -> io::Result<SHA1> {
    SHA1::from_stream(file)
}
