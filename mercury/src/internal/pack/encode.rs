use std::cmp::Ordering;
use std::collections::VecDeque;
use std::io::Write;

use crate::internal::object::types::ObjectType;
use crate::time_it;
use crate::{errors::GitError, hash::SHA1, internal::pack::entry::Entry};
use ahash::AHasher;
use flate2::write::ZlibEncoder;
use rayon::prelude::*;
use sha1::{Digest, Sha1};
use std::hash::{Hash, Hasher};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const MAX_CHAIN_LEN: usize = 50;
const MIN_DELTA_RATE: f64 = 0.5; // minimum delta rate
                                 //const MAX_ZSTDELTA_CHAIN_LEN: usize = 50;

/// A encoder for generating pack files with delta objects.
pub struct PackEncoder {
    object_number: usize,
    process_index: usize,
    window_size: usize,
    // window: VecDeque<(Entry, usize)>, // entry and offset
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

    bytes.push((value & 0x7F) as u8);
    value >>= 7;
    while value != 0 {
        value -= 1;
        let byte = (value & 0x7F) as u8 | 0x80; // set first bit one
        value >>= 7;
        bytes.push(byte);
    }
    bytes.reverse();
    bytes
}

/// Encode one object, and update the hash
/// @offset: offset of this object if it's a delta object. For other object, it's None
fn encode_one_object(entry: &Entry, offset: Option<usize>) -> Result<Vec<u8>, GitError> {
    // try encode as delta
    let obj_data = &entry.data;
    let obj_data_len = obj_data.len();
    let obj_type_number = entry.obj_type.to_u8();

    let mut encoded_data = Vec::new();

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
    encoded_data.extend(header_data);

    // **offset** encoding
    if entry.obj_type == ObjectType::OffsetDelta || entry.obj_type == ObjectType::OffsetZstdelta {
        let offset_data = encode_offset(offset.unwrap());
        encoded_data.extend(offset_data);
    } else if entry.obj_type == ObjectType::HashDelta {
        unreachable!("unsupported type")
    }

    // **data** encoding, need zlib compress
    let mut inflate = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    inflate
        .write_all(obj_data)
        .expect("zlib compress should never failed");
    inflate.flush().expect("zlib flush should never failed");
    let compressed_data = inflate.finish().expect("zlib compress should never failed");
    // self.write_all_and_update(&compressed_data).await;
    encoded_data.extend(compressed_data);
    Ok(encoded_data)
}

fn magic_sort(a: &Entry, b: &Entry) -> Ordering {
    // let ord = b.obj_type.to_u8().cmp(&a.obj_type.to_u8());
    // if ord != Ordering::Equal {
    //     return ord;
    // }

    // the hash should be file hash not content hash
    // todo the feature need larger refactor
    // let ord = b.hash.cmp(&a.hash);
    // if ord != Ordering::Equal { return ord; }

    let ord = b.data.len().cmp(&a.data.len());
    if ord != Ordering::Equal {
        return ord;
    }

    // fallback pointer order (newest first)
    (a as *const Entry).cmp(&(b as *const Entry))
}

// /// 计算单片段 hash
fn calc_hash(data: &[u8]) -> u64 {
    let mut hasher = AHasher::default();
    data.hash(&mut hasher);
    hasher.finish()
}

fn cheap_similar(a: &[u8], b: &[u8]) -> bool {
    let k = a.len().min(b.len()).min(128);
    if k == 0 {
        return false;
    }
    calc_hash(&a[..k]) == calc_hash(&b[..k])
}

impl PackEncoder {
    pub fn new(object_number: usize, window_size: usize, sender: mpsc::Sender<Vec<u8>>) -> Self {
        PackEncoder {
            object_number,
            window_size,
            process_index: 0,
            // window: VecDeque::with_capacity(window_size),
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
    pub async fn encode(&mut self, entry_rx: mpsc::Receiver<Entry>) -> Result<(), GitError> {
        self.inner_encode(entry_rx, false).await
    }

    pub async fn encode_with_zstdelta(
        &mut self,
        entry_rx: mpsc::Receiver<Entry>,
    ) -> Result<(), GitError> {
        return self.inner_encode(entry_rx, true).await;
    }

    /// Delta selection heuristics are based on:
    ///   https://github.com/git/git/blob/master/Documentation/technical/pack-heuristics.adoc
    async fn inner_encode(
        &mut self,
        mut entry_rx: mpsc::Receiver<Entry>,
        enable_zstdelta: bool,
    ) -> Result<(), GitError> {
        let head = encode_header(self.object_number);
        self.send_data(head.clone()).await;
        self.inner_hash.update(&head);

        // ensure only one decode can only invoke once
        if self.start_encoding {
            return Err(GitError::PackEncodeError(
                "encoding operation is already in progress".to_string(),
            ));
        }

        let mut commits: Vec<Entry> = Vec::new();
        let mut trees: Vec<Entry> = Vec::new();
        let mut blobs: Vec<Entry> = Vec::new();
        let mut tags: Vec<Entry> = Vec::new();
        while let Some(entry) = entry_rx.recv().await {
            match entry.obj_type {
                ObjectType::Commit => {
                    commits.push(entry);
                }
                ObjectType::Tree => {
                    trees.push(entry);
                }
                ObjectType::Blob => {
                    blobs.push(entry);
                }
                ObjectType::Tag => {
                    tags.push(entry);
                }
                _ => {}
            }
        }

        commits.sort_by(magic_sort);
        trees.sort_by(magic_sort);
        blobs.sort_by(magic_sort);
        tags.sort_by(magic_sort);
        tracing::info!(
            "numbers :  commits: {:?} trees: {:?} blobs:{:?} tag :{:?}",
            commits.len(),
            trees.len(),
            blobs.len(),
            tags.len()
        );

        // parallel encoding vec with different object_type
        let (commit_results, tree_results, blob_results, tag_results) = tokio::try_join!(
            tokio::task::spawn_blocking(move || {
                Self::try_as_offset_delta(commits, 10, enable_zstdelta)
            }),
            tokio::task::spawn_blocking(move || {
                Self::try_as_offset_delta(trees, 10, enable_zstdelta)
            }),
            tokio::task::spawn_blocking(move || {
                Self::try_as_offset_delta(blobs, 10, enable_zstdelta)
            }),
            tokio::task::spawn_blocking(move || {
                Self::try_as_offset_delta(tags, 10, enable_zstdelta)
            }),
        )
        .map_err(|e| GitError::PackEncodeError(format!("Task join error: {e}")))?;

        // 收集并合并结果
        let all_encoded_data = [
            commit_results
                .map_err(|e| GitError::PackEncodeError(format!("Commit encoding error: {e}")))?,
            tree_results
                .map_err(|e| GitError::PackEncodeError(format!("Tree encoding error: {e}")))?,
            blob_results
                .map_err(|e| GitError::PackEncodeError(format!("Blob encoding error: {e}")))?,
            tag_results
                .map_err(|e| GitError::PackEncodeError(format!("Tag encoding error: {e}")))?,
        ]
        .concat();

        // 按顺序发送合并后的结果
        for data in all_encoded_data {
            self.write_all_and_update(&data).await;
        }

        // Hash signature
        let hash_result = self.inner_hash.clone().finalize();
        self.final_hash = Some(SHA1::from_bytes(&hash_result));
        self.send_data(hash_result.to_vec()).await;

        self.drop_sender();
        Ok(())
    }

    /// Try to encode as delta using objects in window
    /// delta & zstdelta have been gathered here
    /// Refs: https://sapling-scm.com/docs/dev/internals/zstdelta/
    /// the sliding window was moved here
    /// # Returns
    /// - Return (Vec<Vec<u8>) if success make delta
    /// - Return (None) if didn't delta,
    fn try_as_offset_delta(
        mut bucket: Vec<Entry>,
        window_size: usize,
        enable_zstdelta: bool,
    ) -> Result<Vec<Vec<u8>>, GitError> {
        let mut current_offset = 0usize;
        let mut window: VecDeque<(Entry, usize)> = VecDeque::with_capacity(window_size);
        let mut res: Vec<Vec<u8>> = Vec::new();

        for entry in bucket.iter_mut() {
            //let entry_for_window = entry.clone();
            // 每次循环重置最佳基对象选择
            let mut best_base: Option<&(Entry, usize)> = None;
            let mut best_rate: f64 = 0.0;
            let tie_epsilon: f64 = 0.15;

            let candidates: Vec<_> = window
                .par_iter()
                .with_min_len(3)
                .filter_map(|try_base| {
                    if try_base.0.obj_type != entry.obj_type {
                        return None;
                    }

                    if try_base.0.chain_len >= MAX_CHAIN_LEN {
                        return None;
                    }

                    if try_base.0.hash == entry.hash {
                        return None;
                    }

                    let sym_ratio = (try_base.0.data.len().min(entry.data.len()) as f64)
                        / (try_base.0.data.len().max(entry.data.len()) as f64);
                    if sym_ratio < 0.5 {
                        return None;
                    }

                    if !cheap_similar(&try_base.0.data, &entry.data) {
                        return None;
                    }

                    let rate = if (try_base.0.data.len() + entry.data.len()) / 2 > 64 {
                        delta::heuristic_encode_rate_parallel(&try_base.0.data, &entry.data)
                    } else {
                        delta::encode_rate(&try_base.0.data, &entry.data)
                        // let try_delta_obj = zstdelta::diff(&try_base.0.data, &entry.data).unwrap();
                        // 1.0 - try_delta_obj.len() as f64 / entry.data.len() as f64
                    };

                    if rate > MIN_DELTA_RATE {
                        Some((rate, try_base))
                    } else {
                        None
                    }
                })
                .collect();

            for (rate, try_base) in candidates {
                match best_base {
                    None => {
                        best_rate = rate;
                        //best_base_offset = current_offset - try_base.1;
                        best_base = Some(try_base);
                    }
                    Some(best_base_ref) => {
                        let is_better = if rate > best_rate + tie_epsilon {
                            true
                        } else if (rate - best_rate).abs() <= tie_epsilon {
                            try_base.0.chain_len > best_base_ref.0.chain_len
                        } else {
                            false
                        };

                        if is_better {
                            best_rate = rate;
                            best_base = Some(try_base);
                        }
                    }
                }
            }

            let mut entry_for_window = entry.clone();

            let offset = best_base.map(|best_base| {
                let delta = if enable_zstdelta {
                    entry.obj_type = ObjectType::OffsetZstdelta;
                    zstdelta::diff(&best_base.0.data, &entry.data)
                        .map_err(|e| {
                            GitError::DeltaObjectError(format!("zstdelta diff failed: {e}"))
                        })
                        .unwrap()
                } else {
                    entry.obj_type = ObjectType::OffsetDelta;
                    delta::encode(&best_base.0.data, &entry.data)
                };
                //entry.obj_type = ObjectType::OffsetDelta;
                entry.data = delta;
                entry.chain_len = best_base.0.chain_len + 1;
                current_offset - best_base.1
            });

            entry_for_window.chain_len = entry.chain_len;
            let obj_data = encode_one_object(entry, offset)?;
            window.push_back((entry_for_window, current_offset));
            if window.len() > window_size {
                window.pop_front();
            }
            current_offset += obj_data.len();
            res.push(obj_data);
        }
        Ok(res)
    }

    /// Parallel encode with rayon, only works when window_size == 0 (no delta)
    pub async fn parallel_encode(
        &mut self,
        mut entry_rx: mpsc::Receiver<Entry>,
    ) -> Result<(), GitError> {
        if self.window_size != 0 {
            return Err(GitError::PackEncodeError(
                "parallel encode only works when window_size == 0".to_string(),
            ));
        }

        let head = encode_header(self.object_number);
        self.send_data(head.clone()).await;
        self.inner_hash.update(&head);

        // ensure only one decode can only invoke once
        if self.start_encoding {
            return Err(GitError::PackEncodeError(
                "encoding operation is already in progress".to_string(),
            ));
        }

        let batch_size = usize::max(1000, entry_rx.max_capacity() / 10); // A temporary value, not optimized
        tracing::info!("encode with batch size: {}", batch_size);
        loop {
            let mut batch_entries = Vec::with_capacity(batch_size);
            time_it!("parallel encode: receive batch", {
                for _ in 0..batch_size {
                    match entry_rx.recv().await {
                        Some(entry) => {
                            batch_entries.push(entry);
                            self.process_index += 1;
                        }
                        None => break,
                    }
                }
            });

            if batch_entries.is_empty() {
                break;
            }

            // use `collect` will return result in order, refs: https://github.com/rayon-rs/rayon/issues/551#issuecomment-371657900
            let batch_result: Vec<Vec<u8>> = time_it!("parallel encode: encode batch", {
                batch_entries
                    .par_iter()
                    .map(|entry| encode_one_object(entry, None).unwrap())
                    .collect()
            });

            time_it!("parallel encode: write batch", {
                for obj_data in batch_result {
                    self.write_all_and_update(&obj_data).await;
                }
            });
        }

        if self.process_index != self.object_number {
            panic!(
                "not all objects are encoded, process:{}, total:{}",
                self.process_index, self.object_number
            );
        }

        // hash signature
        let hash_result = self.inner_hash.clone().finalize();
        self.final_hash = Some(SHA1::from_bytes(&hash_result));
        self.send_data((hash_result).to_vec()).await;
        self.drop_sender();
        Ok(())
    }

    /// Write data to writer and update hash & offset
    async fn write_all_and_update(&mut self, data: &[u8]) {
        self.inner_hash.update(data);
        self.inner_offset += data.len();
        self.send_data(data.to_vec()).await;
    }

    /// async version of encode, result data will be returned by JoinHandle.
    /// It will consume PackEncoder, so you can't use it after calling this function.
    /// when window_size = 0, it executes parallel_encode which retains stream transmission
    /// when window_size = 0,it executes encode which uses magic sort and delta.
    /// It seems that all other modules rely on this api
    pub async fn encode_async(
        mut self,
        rx: mpsc::Receiver<Entry>,
    ) -> Result<JoinHandle<()>, GitError> {
        Ok(tokio::spawn(async move {
            if self.window_size == 0 {
                self.parallel_encode(rx).await.unwrap()
            } else {
                self.encode(rx).await.unwrap()
            }
        }))
    }

    pub async fn encode_async_with_zstdelta(
        mut self,
        rx: mpsc::Receiver<Entry>,
    ) -> Result<JoinHandle<()>, GitError> {
        Ok(tokio::spawn(async move {
            // Do not use parallel encode with zstdelta because it make no sense.
            self.encode_with_zstdelta(rx).await.unwrap()
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::env;
    use std::sync::Arc;
    use std::time::Instant;
    use std::{io::Cursor, path::PathBuf};
    use tokio::sync::Mutex;

    use crate::internal::object::blob::Blob;
    use crate::internal::pack::utils::read_offset_encoding;
    use crate::internal::pack::{tests::init_logger, Pack};
    use crate::time_it;

    use super::*;

    fn check_format(data: &Vec<u8>) {
        let mut p = Pack::new(
            None,
            Some(1024 * 1024 * 1024 * 6), // 6GB
            Some(PathBuf::from("/tmp/.cache_temp")),
            true,
        );
        let mut reader = Cursor::new(data);
        tracing::debug!("start check format");
        p.decode(&mut reader, |_, _| {})
            .expect("pack file format error");
    }

    #[tokio::test]
    async fn test_pack_encoder() {
        async fn encode_once(window_size: usize) -> Vec<u8> {
            let (tx, mut rx) = mpsc::channel(100);
            let (entry_tx, entry_rx) = mpsc::channel::<Entry>(1);

            // make some different objects, or decode will fail
            let str_vec = vec!["hello, word", "hello, world.", "!", "123141251251"];
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

        // without delta
        let pack_without_delta = encode_once(0).await;
        let pack_without_delta_size = pack_without_delta.len();
        check_format(&pack_without_delta);

        // with delta
        let pack_with_delta = encode_once(4).await;
        assert!(pack_with_delta.len() <= pack_without_delta_size);
        check_format(&pack_with_delta);
    }

    async fn get_entries_for_test() -> Arc<Mutex<Vec<Entry>>> {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-f8bbb573cef7d851957caceb491c073ee8e8de41.pack");
        // let file_map = crate::test_utils::setup_lfs_file().await;
        // let source = file_mapa
        //     .get("git-2d187177923cd618a75da6c6db45bb89d92bd504.pack")
        //     .unwrap();
        // decode pack file to get entries
        let mut p = Pack::new(None, None, Some(PathBuf::from("/tmp/.cache_temp")), true);

        let f = std::fs::File::open(source).unwrap();
        tracing::info!("pack file size: {}", f.metadata().unwrap().len());
        let mut reader = std::io::BufReader::new(f);
        let entries = Arc::new(Mutex::new(Vec::new()));
        let entries_clone = entries.clone();
        p.decode(&mut reader, move |entry, _| {
            let mut entries = entries_clone.blocking_lock();
            entries.push(entry);
        })
        .unwrap();
        assert_eq!(p.number, entries.lock().await.len());
        tracing::info!("total entries: {}", p.number);
        drop(p);

        entries
    }

    #[tokio::test]
    async fn test_pack_encoder_parallel_large_file() {
        init_logger();

        let start = Instant::now(); // 开始时间
        let entries = get_entries_for_test().await;
        let entries_number = entries.lock().await.len();

        // 计算原始总大小
        let total_original_size: usize = entries
            .lock()
            .await
            .iter()
            .map(|entry| entry.data.len())
            .sum();

        // encode entries with parallel
        let (tx, mut rx) = mpsc::channel(1_000_000);
        let (entry_tx, entry_rx) = mpsc::channel::<Entry>(1_000_000);

        let mut encoder = PackEncoder::new(entries_number, 0, tx);
        tokio::spawn(async move {
            time_it!("test parallel encode", {
                encoder.parallel_encode(entry_rx).await.unwrap();
            });
        });

        // spawn a task to send entries
        tokio::spawn(async move {
            let entries = entries.lock().await;
            for entry in entries.iter() {
                entry_tx.send(entry.clone()).await.unwrap();
            }
            drop(entry_tx);
            tracing::info!("all entries sent");
        });

        let mut result = Vec::new();
        while let Some(chunk) = rx.recv().await {
            result.extend(chunk);
        }

        // 计算压缩率
        let pack_size = result.len();
        let compression_rate = if total_original_size > 0 {
            1.0 - (pack_size as f64 / total_original_size as f64)
        } else {
            0.0
        };

        let duration = start.elapsed();
        tracing::info!("test executed in: {:.2?}", duration);
        tracing::info!("new pack file size: {}", result.len());
        tracing::info!("compression rate: {:.2}%", compression_rate * 100.0);
        // check format
        check_format(&result);
    }

    #[tokio::test]
    async fn test_pack_encoder_large_file() {
        init_logger();
        let entries = get_entries_for_test().await;
        let entries_number = entries.lock().await.len();

        // 计算原始总大小
        let total_original_size: usize = entries
            .lock()
            .await
            .iter()
            .map(|entry| entry.data.len())
            .sum();

        let start = Instant::now(); // 开始时间
                                    // encode entries
        let (tx, mut rx) = mpsc::channel(100_000);
        let (entry_tx, entry_rx) = mpsc::channel::<Entry>(100_000);

        let mut encoder = PackEncoder::new(entries_number, 0, tx);
        tokio::spawn(async move {
            time_it!("test encode no parallel", {
                encoder.encode(entry_rx).await.unwrap();
            });
        });

        // spawn a task to send entries
        tokio::spawn(async move {
            let entries = entries.lock().await;
            for entry in entries.iter() {
                entry_tx.send(entry.clone()).await.unwrap();
            }
            drop(entry_tx);
            tracing::info!("all entries sent");
        });

        // // only receive data
        // while (rx.recv().await).is_some() {
        //     // do nothing
        // }

        // 收集数据并计算压缩率
        let mut result = Vec::new();
        while let Some(chunk) = rx.recv().await {
            result.extend(chunk);
        }

        // 计算压缩率
        let pack_size = result.len();
        let compression_rate = if total_original_size > 0 {
            1.0 - (pack_size as f64 / total_original_size as f64)
        } else {
            0.0
        };

        let duration = start.elapsed();
        tracing::info!("test executed in: {:.2?}", duration);
        tracing::info!("new pack file size: {}", pack_size);
        tracing::info!("original total size: {}", total_original_size);
        tracing::info!("compression rate: {:.2}%", compression_rate * 100.0);
        tracing::info!(
            "space saved: {} bytes",
            total_original_size.saturating_sub(pack_size)
        );
    }

    #[tokio::test]
    async fn test_pack_encoder_with_zstdelta() {
        init_logger();
        let entries = get_entries_for_test().await;
        let entries_number = entries.lock().await.len();

        // 计算原始总大小
        let total_original_size: usize = entries
            .lock()
            .await
            .iter()
            .map(|entry| entry.data.len())
            .sum();

        let start = Instant::now();
        let (tx, mut rx) = mpsc::channel(100_000);
        let (entry_tx, entry_rx) = mpsc::channel::<Entry>(100_000);

        let encoder = PackEncoder::new(entries_number, 10, tx);
        encoder.encode_async_with_zstdelta(entry_rx).await.unwrap();

        // spawn a task to send entries
        tokio::spawn(async move {
            let entries = entries.lock().await;
            for entry in entries.iter() {
                entry_tx.send(entry.clone()).await.unwrap();
            }
            drop(entry_tx);
            tracing::info!("all entries sent");
        });

        // 收集数据并计算压缩率
        let mut result = Vec::new();
        while let Some(chunk) = rx.recv().await {
            result.extend(chunk);
        }

        // 计算压缩率
        let pack_size = result.len();
        let compression_rate = if total_original_size > 0 {
            1.0 - (pack_size as f64 / total_original_size as f64)
        } else {
            0.0
        };

        let duration = start.elapsed();
        tracing::info!("test executed in: {:.2?}", duration);
        tracing::info!("new pack file size: {}", pack_size);
        tracing::info!("original total size: {}", total_original_size);
        tracing::info!("compression rate: {:.2}%", compression_rate * 100.0);
        tracing::info!(
            "space saved: {} bytes",
            total_original_size.saturating_sub(pack_size)
        );

        // check format
        check_format(&result);
    }

    #[test]
    fn test_encode_offset() {
        // let value = 11013;
        let value = 16389;

        let data = encode_offset(value);
        println!("{data:?}");
        let mut reader = Cursor::new(data);
        let (result, _) = read_offset_encoding(&mut reader).unwrap();
        println!("result: {result}");
        assert_eq!(result, value as u64);
    }

    #[tokio::test]
    async fn test_pack_encoder_large_file_with_delta() {
        init_logger();
        let entries = get_entries_for_test().await;
        let entries_number = entries.lock().await.len();

        // 计算原始总大小
        let total_original_size: usize = entries
            .lock()
            .await
            .iter()
            .map(|entry| entry.data.len())
            .sum();

        let (tx, mut rx) = mpsc::channel(100_000);
        let (entry_tx, entry_rx) = mpsc::channel::<Entry>(100_000);

        let encoder = PackEncoder::new(entries_number, 10, tx);

        let start = Instant::now(); // 开始时间
        encoder.encode_async(entry_rx).await.unwrap();

        // spawn a task to send entries
        tokio::spawn(async move {
            let entries = entries.lock().await;
            for entry in entries.iter() {
                entry_tx.send(entry.clone()).await.unwrap();
            }
            drop(entry_tx);
            tracing::info!("all entries sent");
        });

        // 收集数据并计算压缩率
        let mut result = Vec::new();
        while let Some(chunk) = rx.recv().await {
            result.extend(chunk);
        }

        // 计算压缩率
        let pack_size = result.len();
        let compression_rate = if total_original_size > 0 {
            1.0 - (pack_size as f64 / total_original_size as f64)
        } else {
            0.0
        };

        let duration = start.elapsed();
        tracing::info!("test executed in: {:.2?}", duration);
        tracing::info!("new pack file size: {}", pack_size);
        tracing::info!("original total size: {}", total_original_size);
        tracing::info!("compression rate: {:.2}%", compression_rate * 100.0);
        tracing::info!(
            "space saved: {} bytes",
            total_original_size.saturating_sub(pack_size)
        );

        // check format
        check_format(&result);
    }
}
