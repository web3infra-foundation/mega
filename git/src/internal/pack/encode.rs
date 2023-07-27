use sha1::{Digest, Sha1};
use std::io::{Cursor, Write};
use std::sync::Arc;

use crate::internal::object::ObjectT;
use crate::internal::zlib::stream::deflate::Write as Writer;

use std::io::Error;
#[allow(unused)]
struct Encoder<W> {
    inner: W,
    hash: Sha1,
}
#[allow(unused)]
impl<W> Encoder<W>
where
    W: Write,
{
    pub fn init(object_number: usize, mut inner: W) -> Self {
        let head = encode_header(object_number);
        inner.write_all(&head).unwrap();
        let mut hash = Sha1::new();
        hash.update(&head);
        Self { inner, hash }
    }
    pub fn add_objects(&mut self, obj_vec: Vec<Arc<dyn ObjectT>>) -> Result<(), Error> {
        for obj in obj_vec {
            let obj_data = encode_one_object(obj)?;
            self.hash.update(&obj_data);
            self.inner.write_all(&obj_data)?;
        }
        Ok(())
    }
    pub fn finish(&mut self) -> Result<(), Error> {
        let hash_result = self.hash.clone().finalize();
        self.inner.write_all(&hash_result)?;
        Ok(())
    }
}
//

pub fn pack_encode(obj_vec: Vec<Arc<dyn ObjectT>>) -> Result<Vec<u8>, Error> {
    let mut hash = Sha1::new();
    let mut out_data = Vec::new();
    let header_data = encode_header(obj_vec.len());
    hash.update(&header_data);
    out_data.write_all(&header_data)?;

    for obj in obj_vec {
        let obj_data = encode_one_object(obj)?;
        hash.update(&obj_data);
        out_data.write_all(&obj_data)?;
    }
    let hash_result = hash.finalize();
    out_data.write_all(&hash_result)?;
    Ok(out_data)
}

fn encode_header(object_number: usize) -> Vec<u8> {
    let mut result: Vec<u8> = vec![
        b'P', b'A', b'C', b'K', // The logotype of the Pack File
        0, 0, 0, 2,
    ]; // THe Version  of the Pack File
    assert_ne!(object_number, 0); // guarantee self.number_of_objects!=0
    assert!(object_number < (1 << 32));
    //TODO: GitError:numbers of objects should  < 4G ,
    //Encode the number of object  into file
    result.append(&mut u32_vec(object_number as u32));
    result
}

fn encode_one_object(obj: Arc<dyn ObjectT>) -> Result<Vec<u8>, Error> {
    let mut out = Writer::new(Vec::new());
    let obj_data = obj.get_raw();
    let size = obj_data.len();
    let mut header_data = vec![(0x80 | (obj.get_type().type2number() << 4)) + (size & 0x0f) as u8];
    let mut _size = size >> 4;
    if _size > 0 {
        while _size > 0 {
            if _size >> 7 > 0 {
                header_data.push((0x80 | _size) as u8);
                _size >>= 7;
            } else {
                header_data.push((_size) as u8);
                break;
            }
        }
    } else {
        header_data.push(0);
    }

    if let Err(err) = std::io::copy(&mut Cursor::new(obj_data), &mut out) {
        match err.kind() {
            std::io::ErrorKind::Other => return Err(err),
            err => {
                unreachable!("Should never see other errors than zlib, but got {:?}", err,)
            }
        }
    };
    out.flush().expect("zlib flush should never fail");
    header_data.append(&mut out.into_inner());
    Ok(header_data)
}

fn u32_vec(value: u32) -> Vec<u8> {
    vec![
        (value >> 24 & 0xff) as u8,
        (value >> 16 & 0xff) as u8,
        (value >> 8 & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

#[cfg(test)]
mod tests {

    use tokio_test::block_on;

    use crate::{
        hash::Hash,
        internal::{
            object::{blob::Blob, ObjectT},
            pack::Pack,
        },
    };
    use std::io::Cursor;
    use std::sync::Arc;

    use super::{pack_encode, Encoder};

    #[test]
    fn test_a_simple_encode() {
        let id = Hash([0u8; 20]);
        let data = String::from("hello,1").into_bytes();
        let mut obj_vec: Vec<Arc<dyn ObjectT>> = Vec::new();
        let b1 = Blob { id, data };
        obj_vec.push(Arc::new(b1));
        let data = String::from("hello,2").into_bytes();
        let b2 = Blob { id, data };
        obj_vec.push(Arc::new(b2));

        let result = pack_encode(obj_vec).unwrap();
        let mut buff = Cursor::new(result);
        block_on(Pack::decode(&mut buff)).unwrap();
    }

    #[test]
    fn test_pack_encoder() {
        let id = Hash([0u8; 20]);
        let mut pack_data = Vec::with_capacity(1000);
        // Encoder::init
        let mut encoder = Encoder::init(2, &mut pack_data);
        let mut obj_vec: Vec<Arc<dyn ObjectT>> = Vec::new();
        let data = String::from("hello,1").into_bytes();
        let b1 = Blob { id, data };
        obj_vec.push(Arc::new(b1));
        let data = String::from("hello,2").into_bytes();
        let b2 = Blob { id, data };
        obj_vec.push(Arc::new(b2));
        // Encoder::add_objects
        encoder.add_objects(obj_vec).unwrap();
        // Encoder::finish
        encoder.finish().unwrap();
        let mut buff = Cursor::new(pack_data);
        block_on(Pack::decode(&mut buff)).unwrap();
    }
}
