use std::io;
use std::io::Read;

pub fn read_u32_be(file: &mut impl Read) -> io::Result<u32> {
    let mut buf = [0; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}