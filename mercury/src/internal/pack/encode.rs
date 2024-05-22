use std::collections::VecDeque;
use std::io::Write;

use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::internal::object::types::ObjectType;
use crate::{errors::GitError, hash::SHA1, internal::pack::entry::Entry};

const MIN_DELTA_RATE: f64 = 0.5; // minimum delta rate can accept

/// A encoder for generating pack files with delta objects.
pub struct PackEncoder {
    object_number: usize,
    process_index: usize,
    window_size: usize,
    window: VecDeque<(Entry, usize)>, // entry and offset
    sender: Option<mpsc::Sender<Vec<u8>>>,
    inner_offset: usize, // offset of current entry
    inner_hash: Sha1,    // Not SHA1 because need update trait
    final_hash: Option<SHA1>,
    start_encoding: bool,
}

/// Encode header of pack file (12 byte)<br>
/// Content: 'PACK', Version(2), number of objects
fn encode_header(object_number: usize) -> Vec<u8> {
    let mut result: Vec<u8> = vec![
        b'P', b'A', b'C', b'K', // The logotype of the Pack File
        0, 0, 0, 2, // generates version 2 only.
    ];
    assert_ne!(object_number, 0); // guarantee self.number_of_objects!=0
    assert!(object_number < (1 << 32));
    //TODO: GitError:numbers of objects should < 4G ,
    result.append((object_number as u32).to_be_bytes().to_vec().as_mut()); // to 4 bytes (network byte order aka. big-endian)
    result
}

/// Encode offset of delta object
fn encode_offset(mut value: usize) -> Vec<u8> {
    assert_ne!(value, 0, "offset can't be zero");
    let mut bytes = Vec::new();
    let mut first_byte = true;
    while value != 0 || first_byte {
        let mut byte = (value & 0x7F) as u8; // 获取当前值的最低7位
        value >>= 7; // 右移7位准备处理下一个字节
        if first_byte {
            first_byte = false;
        } else {
            byte -= 1; // sub 1
            byte |= 0x80; // set first bit one
        }
        bytes.push(byte);
    }
    bytes.reverse();
    bytes
}

impl PackEncoder {
    pub fn new(object_number: usize, window_size: usize, sender: mpsc::Sender<Vec<u8>>) -> Self {
        PackEncoder {
            object_number,
            window_size,
            process_index: 0,
            window: VecDeque::with_capacity(window_size),
            sender: Some(sender),
            inner_offset: 12, // 12 bytes header
            inner_hash: Sha1::new(),
            final_hash: None,
            start_encoding: false,
        }
    }

    pub fn drop_sender(&mut self) {
        self.sender.take(); // Take the sender out, dropping it
    }

    pub async fn send_data(&mut self, data: Vec<u8>) {
        if let Some(sender) = &self.sender {
            sender.send(data).await.unwrap();
        }
    }

    /// Get the hash of the pack file. if the pack file is not finished, return None
    pub fn get_hash(&self) -> Option<SHA1> {
        self.final_hash
    }

    /// Encodes entries into a pack file with delta objects and outputs them through the specified writer.
    /// # Arguments
    /// - `rx` - A receiver channel (`mpsc::Receiver<Entry>`) from which entries to be encoded are received.
    /// # Returns
    /// Returns `Ok(())` if encoding is successful, or a `GitError` in case of failure.
    /// - Returns a `GitError` if there is a failure during the encoding process.
    /// - Returns `PackEncodeError` if an encoding operation is already in progress.
    pub async fn encode(&mut self, mut entry_rx: mpsc::Receiver<Entry>) -> Result<(), GitError> {
        let head = encode_header(self.object_number);
        self.send_data(head.clone()).await;
        self.inner_hash.update(&head);

        // ensure only one decode can only invoke once
        if self.start_encoding {
            return Err(GitError::PackEncodeError(
                "encoding operation is already in progress".to_string(),
            ));
        }
        loop {
            match entry_rx.recv().await {
                Some(entry) => {
                    self.process_index += 1;
                    // push window after encode to void diff by self
                    let offset = self.inner_offset;
                    self.encode_one_object(&entry).await?;
                    self.window.push_back((entry, offset));
                    if self.window.len() > self.window_size {
                        self.window.pop_front();
                    }
                }
                None => {
                    if self.process_index != self.object_number {
                        panic!("not all objects are encoded");
                    }
                    break;
                }
            }
        }

        // hash signature
        let hash_result = self.inner_hash.clone().finalize();
        self.final_hash = Some(SHA1::from_bytes(&hash_result));
        self.send_data((hash_result).to_vec()).await;
        self.drop_sender();
        Ok(())
    }

    /// Try to encode as delta using objects in window
    /// # Returns
    /// - Return (delta entry, offset) if success make delta
    /// - Return (origin Entry,None) if didn't delta,
    fn try_as_offset_delta(&mut self, entry: &Entry) -> (Entry, Option<usize>) {
        let mut best_base: Option<&(Entry, usize)> = None;
        let mut best_rate: f64 = 0.0;
        for try_base in self.window.iter() {
            if try_base.0.obj_type != entry.obj_type {
                continue;
            }
            let rate = delta::encode_rate(&try_base.0.data, &entry.data);
            if rate > MIN_DELTA_RATE && rate > best_rate {
                best_rate = rate;
                best_base = Some(try_base);
            }
        }
        if best_rate > 0.0 {
            let best_base = best_base.unwrap(); // must some if best rate > 0
            let delta = delta::encode(&best_base.0.data, &entry.data);
            let offset = self.inner_offset - best_base.1;
            (
                Entry {
                    data: delta,
                    obj_type: ObjectType::OffsetDelta,
                    ..entry.clone()
                },
                Some(offset),
            )
        } else {
            (entry.clone(), None)
        }
    }

    /// Write data to writer and update hash & offset
    async fn write_all_and_update(&mut self, data: &[u8]) {
        self.inner_hash.update(data);
        self.inner_offset += data.len();
        self.send_data(data.to_vec()).await;
    }

    /// Encode one object, and update the hash
    async fn encode_one_object(&mut self, entry: &Entry) -> Result<(), GitError> {
        // try encode as delta
        let (entry, offset) = self.try_as_offset_delta(entry);
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
                    header_data.push(size as u8);
                    break;
                }
            }
        } else {
            header_data.push(0);
        }
        self.write_all_and_update(&header_data).await;

        // **offset** encoding
        if entry.obj_type == ObjectType::OffsetDelta {
            let offset_data = encode_offset(offset.unwrap());
            self.write_all_and_update(&offset_data).await;
        } else if entry.obj_type == ObjectType::HashDelta {
            unreachable!("unsupported type")
        }

        // **data** encoding, need zlib compress
        let mut inflate = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        inflate
            .write_all(&obj_data)
            .expect("zlib compress should never failed");
        inflate.flush().expect("zlib flush should never failed");
        let compressed_data = inflate.finish().expect("zlib compress should never failed");
        self.write_all_and_update(&compressed_data).await;
        Ok(())
    }

    /// async version of encode, result data will be returned by JoinHandle.
    /// It will consume PackEncoder, so you can't use it after calling this function.
    pub async fn encode_async(
        mut self,
        rx: mpsc::Receiver<Entry>,
    ) -> Result<JoinHandle<()>, GitError> {
        Ok(tokio::spawn(async move { self.encode(rx).await.unwrap() }))
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::PathBuf, usize};

    use crate::internal::object::blob::Blob;
    use crate::internal::pack::Pack;

    use super::*;

    #[tokio::test]
    async fn test_pack_encoder() {
        async fn encode_once(window_size: usize) -> Vec<u8> {
            let (tx, mut rx) = mpsc::channel(100);
            let (entry_tx, entry_rx) = mpsc::channel::<Entry>(1);

            // make some different objects, or decode will fail
            let str_vec = vec!["hello, code,", "hello, world.", "!", "123141251251"];
            let encoder = PackEncoder::new(str_vec.len(), window_size, tx);
            encoder.encode_async(entry_rx).await.unwrap();

            for str in str_vec {
                let blob = Blob::from_content(str);
                let entry: Entry = blob.into();
                entry_tx.send(entry).await.unwrap();
            }
            drop(entry_tx);
            // assert!(encoder.get_hash().is_some());
            let mut result = Vec::new();
            while let Some(chunk) = rx.recv().await {
                result.extend(chunk);
            }
            result
        }

        fn check_format(data: Vec<u8>) {
            let mut p = Pack::new(
                None,
                Some(1024 * 20),
                Some(PathBuf::from("/tmp/.cache_temp")),
                true,
            );
            let mut reader = Cursor::new(data);
            p.decode(&mut reader, |_, _| {})
                .expect("pack file format error");
        }
        // without delta
        let pack_without_delta = encode_once(0).await;
        let pack_without_delta_size = pack_without_delta.len();
        check_format(pack_without_delta);

        // with delta
        let pack_with_delta = encode_once(3).await;
        assert_ne!(pack_with_delta.len(), pack_without_delta_size);
        check_format(pack_with_delta);
    }

    #[test]
    fn test_encode_offset() {
        let value = 11013;
        let data = encode_offset(value);
        println!("{:?}", data);
        assert_eq!(data.len(), 2);
        assert_eq!(data[0], 0b_1101_0101);
        assert_eq!(data[1], 0b_0000_0101);
    }

}
