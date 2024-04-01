use std::io;
use std::io::Read;
use venus::hash::SHA1;

pub const SHA1_SIZE: usize = 20;
pub fn read_u32_be(file: &mut impl Read) -> io::Result<u32> {
    let mut buf = [0; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub fn read_u16_be(file: &mut impl Read) -> io::Result<u16> {
    let mut buf = [0; 2];
    file.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

pub fn read_bytes(file: &mut impl Read, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_sha1(file: &mut impl Read) -> io::Result<SHA1> {
    let mut buf = [0; 20];
    file.read_exact(&mut buf)?;
    Ok(SHA1::from_bytes(&buf))
}