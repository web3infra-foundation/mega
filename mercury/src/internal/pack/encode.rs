//!
//!
//!

use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::collections::VecDeque;
use std::{io::Write, sync::mpsc};
use venus::{errors::GitError, hash::SHA1, internal::pack::entry::Entry};

pub struct PackEncoder<W: Write> {
    object_number: usize,
    process_index: usize,
    window_size: usize,
    window: VecDeque<Entry>,
    writer: W,
    innser_hash: Sha1, // Not SHA1 because need update trait
    final_hash: Option<SHA1>,
}
fn u32_vec(value: u32) -> Vec<u8> {
    vec![
        (value >> 24 & 0xff) as u8,
        (value >> 16 & 0xff) as u8,
        (value >> 8 & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}
fn encode_header(object_number: usize) -> Vec<u8> {
    let mut result: Vec<u8> = vec![
        b'P', b'A', b'C', b'K', // The logotype of the Pack File
        0, 0, 0, 2,
    ]; // THe Version  of the Pack File
    assert_ne!(object_number, 0); // guarantee self.number_of_objects!=0
    assert!(object_number < (1 << 32));
    //TODO: GitError:numbers of objects should  < 4G ,
    result.append(&mut u32_vec(object_number as u32));
    result
}

impl<W: Write> PackEncoder<W> {
    pub fn new(object_number: usize, window_size: usize, mut writer: W) -> Self {
        let head = encode_header(object_number);
        writer.write_all(&head).unwrap();
        let mut hash = Sha1::new();
        hash.update(&head);
        PackEncoder {
            object_number,
            window_size,
            process_index: 0,
            window: VecDeque::with_capacity(window_size),
            writer,
            innser_hash: hash,
            final_hash: None,
        }
    }

    /// get the hash of the pack file. if the pack file is not finished, return None
    pub fn get_hash(&self) -> Option<SHA1> {
        self.final_hash
    }

    /// encode a pack file, write to inner writer
    pub fn encode(&mut self, rx: mpsc::Receiver<Entry>) -> Result<(), GitError> {
        while match rx.recv() {
            Ok(entry) => {
                self.process_index += 1;
                self.encode_one_object(&entry)?;
                self.window.push_back(entry);
                if self.window.len() > self.window_size {
                    let _ = self.window.pop_front().unwrap();
                }
                true
            }
            Err(_) => {
                if self.process_index != self.object_number {
                    panic!("not all objects are encoded");
                }
                false
            }
        } {}

        // hash signature
        let hash_result = self.innser_hash.clone().finalize();
        self.final_hash = Some(SHA1::new(&hash_result.to_vec()));
        self.writer.write_all(&hash_result).unwrap();
        Ok(())
    }
    /// try encode as delta
    fn try_as_delta(&mut self, entry: &Entry) -> Result<Option<Entry>, GitError> {
        let _ = entry;
        // TODO no delta support now
        Ok(None)
    }
    fn write_all_and_update_hash(&mut self, data: &[u8]) {
        self.innser_hash.update(data);
        self.writer.write_all(data).unwrap();
    }

    /// encode one object, and update the hash
    fn encode_one_object(&mut self, entry: &Entry) -> Result<(), GitError> {
        // try encode as delta
        let entry = match self.try_as_delta(entry)? {
            Some(entry) => entry,
            None => entry.clone(),
        };

        let obj_data = entry.data;
        let obj_data_len = obj_data.len();
        let obj_type_number = entry.obj_type.to_u8();

        // **header** encoding
        let mut header_data = vec![(0x80 | (obj_type_number << 4)) + (obj_data_len & 0x0f) as u8];
        let mut size = obj_data_len >> 4; // 4 bit has been used in first byte
        if size > 0 {
            while size > 0 {
                if size >> 7 > 0 {
                    header_data.push((0x80 | size) as u8);
                    size >>= 7;
                } else {
                    header_data.push((size) as u8);
                    break;
                }
            }
        } else {
            header_data.push(0);
        }
        self.write_all_and_update_hash(&header_data);

        // **delta** encoding
        if !entry.obj_type.is_base() {
            todo!("delta encoding"); // TODO
        }

        let mut inflate = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        // **data** encoding, need zlib compress
        inflate
            .write_all(&obj_data)
            .expect("zlib compress should never failed");
        inflate.flush().expect("zlib flush should never failed");
        let compressed_data = inflate.finish().expect("zlib compress should never failed");
        self.write_all_and_update_hash(&compressed_data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::PathBuf};

    use venus::internal::object::blob::Blob;

    use crate::internal::pack::Pack;

    use super::*;
    #[test]
    fn test_pack_encoder_without_delta() {
        let mut writter: Vec<u8> = Vec::new();
        let mut encoder = PackEncoder::new(3, 0, &mut writter);
        let (tx, rx) = mpsc::channel::<Entry>();

        // make some different objects, or decode will fail
        let str_vec = vec!["hello", "world", "!"];
        for str in str_vec {
            let blob = Blob::from_content(str);
            let entry: Entry = blob.into();
            tx.send(entry).unwrap();
        }
        drop(tx);
        encoder.encode(rx).unwrap();
        assert!(encoder.get_hash().is_some());

        // use decode to check the pack file
        let mut p = Pack::new(
            None,
            Some(1024 * 20),
            Some(PathBuf::from("/tmp/.cache_temp")),
        );
        let mut reader = Cursor::new(writter);
        p.decode(&mut reader, None).expect("pack file format error");
    }
}
