use std::io::{self, BufRead, Cursor, ErrorKind, Read};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use axum::Error;
use bytes::Bytes;
use flate2::bufread::ZlibDecoder;
use futures_util::{Stream, StreamExt};
use threadpool::ThreadPool;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::errors::GitError;
use crate::hash::SHA1;
use crate::internal::object::types::ObjectType;

use crate::internal::pack::cache::Caches;
use crate::internal::pack::cache::_Cache;
use crate::internal::pack::cache_object::{CacheObject, MemSizeRecorder};
use crate::internal::pack::channel_reader::StreamBufReader;
use crate::internal::pack::entry::Entry;
use crate::internal::pack::waitlist::Waitlist;
use crate::internal::pack::wrapper::Wrapper;
use crate::internal::pack::{utils, Pack, DEFAULT_TMP_DIR};

use super::cache_object::CacheObjectInfo;

/// For the convenience of passing parameters
struct SharedParams {
    pub pool: Arc<ThreadPool>,
    pub waitlist: Arc<Waitlist>,
    pub caches: Arc<Caches>,
    pub cache_objs_mem_size: Arc<AtomicUsize>,
    pub callback: Arc<dyn Fn(Entry, usize) + Sync + Send>,
}

impl Drop for Pack {
    fn drop(&mut self) {
        if self.clean_tmp {
            self.caches.remove_tmp_dir();
        }
    }
}

impl Pack {
    /// # Parameters
    /// - `thread_num`: The number of threads to use for decoding and cache, `None` mean use the number of logical CPUs.
    ///   It can't be zero, or panic <br>
    /// - `mem_limit`: The maximum size of the memory cache in bytes, or None for unlimited.
    ///   The 80% of it will be used for [Caches]  <br>
    ///     **Not very accurate, because of memory alignment and other reasons, overuse about 15%** <br>
    /// - `temp_path`: The path to a directory for temporary files, default is "./.cache_temp" <br>
    ///   For example, thread_num = 4 will use up to 8 threads (4 for decoding and 4 for cache) <br>
    /// - `clean_tmp`: whether to remove temp directory when Pack is dropped
    pub fn new(
        thread_num: Option<usize>,
        mem_limit: Option<usize>,
        temp_path: Option<PathBuf>,
        clean_tmp: bool,
    ) -> Self {
        let mut temp_path = temp_path.unwrap_or(PathBuf::from(DEFAULT_TMP_DIR));
        // add 8 random characters as subdirectory, check if the directory exists
        loop {
            let sub_dir = Uuid::new_v4().to_string()[..8].to_string();
            temp_path.push(sub_dir);
            if !temp_path.exists() {
                break;
            }
            temp_path.pop();
        }
        let thread_num = thread_num.unwrap_or_else(num_cpus::get);
        let cache_mem_size = mem_limit.map(|mem_limit| mem_limit * 4 / 5);
        Pack {
            number: 0,
            signature: SHA1::default(),
            objects: Vec::new(),
            pool: Arc::new(ThreadPool::new(thread_num)),
            waitlist: Arc::new(Waitlist::new()),
            caches: Arc::new(Caches::new(cache_mem_size, temp_path, thread_num)),
            mem_limit,
            cache_objs_mem: Arc::new(AtomicUsize::default()),
            clean_tmp,
        }
    }

    /// Checks and reads the header of a Git pack file.
    ///
    /// This function reads the first 12 bytes of a pack file, which include the b"PACK" magic identifier,
    /// the version number, and the number of objects in the pack. It verifies that the magic identifier
    /// is correct and that the version number is 2 (which is the version currently supported by Git).
    /// It also collects these header bytes for later use, such as for hashing the entire pack file.
    ///
    /// # Parameters
    /// * `pack`: A mutable reference to an object implementing the `Read` trait,
    ///           representing the source of the pack file data (e.g., file, memory stream).
    ///
    /// # Returns
    /// A `Result` which is:
    /// * `Ok((u32, Vec<u8>))`: On successful reading and validation of the header, returns a tuple where:
    ///     - The first element is the number of objects in the pack file (`u32`).
    ///     - The second element is a vector containing the bytes of the pack file header (`Vec<u8>`).
    /// * `Err(GitError)`: On failure, returns a [`GitError`] with a description of the issue.
    ///
    /// # Errors
    /// This function can return an error in the following situations:
    /// * If the pack file does not start with the "PACK" magic identifier.
    /// * If the pack file's version number is not 2.
    /// * If there are any issues reading from the provided `pack` source.
    pub fn check_header(pack: &mut impl BufRead) -> Result<(u32, Vec<u8>), GitError> {
        // A vector to store the header data for hashing later
        let mut header_data = Vec::new();

        // Read the first 4 bytes which should be "PACK"
        let mut magic = [0; 4];
        // Read the magic "PACK" identifier
        let result = pack.read_exact(&mut magic);
        match result {
            Ok(_) => {
                // Store these bytes for later
                header_data.extend_from_slice(&magic);

                // Check if the magic bytes match "PACK"
                if magic != *b"PACK" {
                    // If not, return an error indicating invalid pack header
                    return Err(GitError::InvalidPackHeader(format!(
                        "{},{},{},{}",
                        magic[0], magic[1], magic[2], magic[3]
                    )));
                }
            }
            Err(e) => {
                // If there is an error in reading, return a GitError
                return Err(GitError::InvalidPackFile(format!(
                    "Error reading magic identifier: {}",
                    e
                )));
            }
        }

        // Read the next 4 bytes for the version number
        let mut version_bytes = [0; 4];
        let result = pack.read_exact(&mut version_bytes); // Read the version number
        match result {
            Ok(_) => {
                // Store these bytes
                header_data.extend_from_slice(&version_bytes);

                // Convert the version bytes to an u32 integer
                let version = u32::from_be_bytes(version_bytes);
                if version != 2 {
                    // Git currently supports version 2, so error if not version 2
                    return Err(GitError::InvalidPackFile(format!(
                        "Version Number is {}, not 2",
                        version
                    )));
                }
            }
            Err(e) => {
                // If there is an error in reading, return a GitError
                return Err(GitError::InvalidPackFile(format!(
                    "Error reading version number: {}",
                    e
                )));
            }
        }

        // Read the next 4 bytes for the number of objects in the pack
        let mut object_num_bytes = [0; 4];
        // Read the number of objects
        let result = pack.read_exact(&mut object_num_bytes);
        match result {
            Ok(_) => {
                // Store these bytes
                header_data.extend_from_slice(&object_num_bytes);
                // Convert the object number bytes to an u32 integer
                let object_num = u32::from_be_bytes(object_num_bytes);
                // Return the number of objects and the header data for further processing
                Ok((object_num, header_data))
            }
            Err(e) => {
                // If there is an error in reading, return a GitError
                Err(GitError::InvalidPackFile(format!(
                    "Error reading object number: {}",
                    e
                )))
            }
        }
    }

    /// Decompresses data from a given Read and BufRead source using Zlib decompression.
    ///
    /// # Parameters
    /// * `pack`: A source that implements both Read and BufRead traits (e.g., file, network stream).
    /// * `expected_size`: The expected decompressed size of the data.
    ///
    /// # Returns
    /// Returns a `Result` containing either:
    /// * A tuple with a `Vec<u8>` of the decompressed data and the total number of input bytes processed,
    /// * Or a `GitError` in case of a mismatch in expected size or any other reading error.
    ///
    pub fn decompress_data(
        &mut self,
        pack: &mut (impl BufRead + Send),
        expected_size: usize,
    ) -> Result<(Vec<u8>, usize), GitError> {
        // Create a buffer with the expected size for the decompressed data
        let mut buf = Vec::with_capacity(expected_size);
        // Create a new Zlib decoder with the original data
        let mut deflate = ZlibDecoder::new(pack);

        // Attempt to read data to the end of the buffer
        match deflate.read_to_end(&mut buf) {
            Ok(_) => {
                // Check if the length of the buffer matches the expected size
                if buf.len() != expected_size {
                    Err(GitError::InvalidPackFile(format!(
                        "The object size {} does not match the expected size {}",
                        buf.len(),
                        expected_size
                    )))
                } else {
                    // If everything is as expected, return the buffer, the original data, and the total number of input bytes processed
                    Ok((buf, deflate.total_in() as usize))
                    // TODO this will likely be smaller than what the decompressor actually read from the underlying stream due to buffering.
                }
            }
            Err(e) => {
                // If there is an error in reading, return a GitError
                Err(GitError::InvalidPackFile(format!(
                    "Decompression error: {}",
                    e
                )))
            }
        }
    }

    /// Decodes a pack object from a given Read and BufRead source and returns the object as a [`CacheObject`].
    ///
    /// # Parameters
    /// * `pack`: A source that implements both Read and BufRead traits.
    /// * `offset`: A mutable reference to the current offset within the pack.
    ///
    /// # Returns
    /// Returns a `Result` containing either:
    /// * A tuple of the next offset in the pack and the original compressed data as `Vec<u8>`,
    /// * Or a `GitError` in case of any reading or decompression error.
    ///
    pub fn decode_pack_object(
        &mut self,
        pack: &mut (impl BufRead + Send),
        offset: &mut usize,
    ) -> Result<CacheObject, GitError> {
        let init_offset = *offset;

        // Attempt to read the type and size, handle potential errors
        let (type_bits, size) = match utils::read_type_and_varint_size(pack, offset) {
            Ok(result) => result,
            Err(e) => {
                // Handle the error e.g., by logging it or converting it to GitError
                // and then return from the function
                return Err(GitError::InvalidPackFile(format!("Read error: {}", e)));
            }
        };

        // Check if the object type is valid
        let t = ObjectType::from_u8(type_bits)?;

        match t {
            ObjectType::Commit | ObjectType::Tree | ObjectType::Blob | ObjectType::Tag => {
                let (data, raw_size) = self.decompress_data(pack, size)?;
                *offset += raw_size;
                Ok(CacheObject::new_for_undeltified(t, data, init_offset))
            }
            ObjectType::OffsetDelta => {
                let (delta_offset, bytes) = utils::read_offset_encoding(pack).unwrap();
                *offset += bytes;

                let (data, raw_size) = self.decompress_data(pack, size)?;
                *offset += raw_size;

                // Count the base object offset: the current offset - delta offset
                let base_offset = init_offset
                    .checked_sub(delta_offset as usize)
                    .ok_or_else(|| {
                        GitError::InvalidObjectInfo("Invalid OffsetDelta offset".to_string())
                    })
                    .unwrap();

                let mut reader = Cursor::new(&data);
                let (_, final_size) = utils::read_delta_object_size(&mut reader)?;

                Ok(CacheObject {
                    info: CacheObjectInfo::OffsetDelta(base_offset, final_size),
                    offset: init_offset,
                    data_decompressed: data,
                    mem_recorder: None,
                })
            }
            ObjectType::HashDelta => {
                // Read 20 bytes to get the reference object SHA1 hash
                let ref_sha1 = SHA1::from_stream(pack).unwrap();
                // Offset is incremented by 20 bytes
                *offset += SHA1::SIZE;

                let (data, raw_size) = self.decompress_data(pack, size)?;
                *offset += raw_size;

                let mut reader = Cursor::new(&data);
                let (_, final_size) = utils::read_delta_object_size(&mut reader)?;

                Ok(CacheObject {
                    info: CacheObjectInfo::HashDelta(ref_sha1, final_size),
                    offset: init_offset,
                    data_decompressed: data,
                    mem_recorder: None,
                })
            }
        }
    }

    /// Decodes a pack file from a given Read and BufRead source, for each object in the pack,
    /// it decodes the object and processes it using the provided callback function.
    pub fn decode<F>(
        &mut self,
        pack: &mut (impl BufRead + Send),
        callback: F,
    ) -> Result<(), GitError>
    where
        F: Fn(Entry, usize) + Sync + Send + 'static,
    {
        let time = Instant::now();
        let mut last_update_time = time.elapsed().as_millis();
        let log_info = |_i: usize, pack: &Pack| {
            tracing::info!("time {:.2} s \t decode: {:?} \t dec-num: {} \t cah-num: {} \t Objs: {} MB \t CacheUsed: {} MB",
                time.elapsed().as_millis() as f64 / 1000.0, _i, pack.pool.queued_count(), pack.caches.queued_tasks(),
                pack.cache_objs_mem_used() / 1024 / 1024,
                pack.caches.memory_used() / 1024 / 1024);
        };
        let callback = Arc::new(callback);

        let caches = self.caches.clone();
        let mut reader = Wrapper::new(io::BufReader::new(pack));

        let result = Pack::check_header(&mut reader);
        match result {
            Ok((object_num, _)) => {
                self.number = object_num as usize;
            }
            Err(e) => {
                return Err(e);
            }
        }
        tracing::info!("The pack file has {} objects", self.number);
        let mut offset: usize = 12;
        let mut i = 0;
        while i < self.number {
            // log per 1000 objects and 1 second
            if i % 1000 == 0 {
                let time_now = time.elapsed().as_millis();
                if time_now - last_update_time > 1000 {
                    log_info(i, self);
                    last_update_time = time_now;
                }
            }
            // 3 parts: Waitlist + TheadPool + Caches
            // hardcode the limit of the tasks of threads_pool queue, to limit memory
            while self.pool.queued_count() > 2000
                || self
                    .mem_limit
                    .map(|limit| self.memory_used() > limit)
                    .unwrap_or(false)
            {
                thread::yield_now();
            }
            let r: Result<CacheObject, GitError> =
                self.decode_pack_object(&mut reader, &mut offset);
            match r {
                Ok(mut obj) => {
                    obj.set_mem_recorder(self.cache_objs_mem.clone());
                    obj.record_mem_size();

                    // Wrapper of Arc Params, for convenience to pass
                    let params = Arc::new(SharedParams {
                        pool: self.pool.clone(),
                        waitlist: self.waitlist.clone(),
                        caches: self.caches.clone(),
                        cache_objs_mem_size: self.cache_objs_mem.clone(),
                        callback: callback.clone(),
                    });

                    let caches = caches.clone();
                    let waitlist = self.waitlist.clone();
                    self.pool.execute(move || {
                        match obj.info {
                            CacheObjectInfo::BaseObject(_, _) => {
                                Self::cache_obj_and_process_waitlist(params, obj);
                            }
                            CacheObjectInfo::OffsetDelta(base_offset, _) => {
                                if let Some(base_obj) = caches.get_by_offset(base_offset) {
                                    Self::process_delta(params, obj, base_obj);
                                } else {
                                    // You can delete this 'if' block ↑, because there are Second check in 'else'
                                    // It will be more readable, but the performance will be slightly reduced
                                    waitlist.insert_offset(base_offset, obj);
                                    // Second check: prevent that the base_obj thread has finished before the waitlist insert
                                    if let Some(base_obj) = caches.get_by_offset(base_offset) {
                                        Self::process_waitlist(params, base_obj);
                                    }
                                }
                            }
                            CacheObjectInfo::HashDelta(base_ref, _) => {
                                if let Some(base_obj) = caches.get_by_hash(base_ref) {
                                    Self::process_delta(params, obj, base_obj);
                                } else {
                                    waitlist.insert_ref(base_ref, obj);
                                    if let Some(base_obj) = caches.get_by_hash(base_ref) {
                                        Self::process_waitlist(params, base_obj);
                                    }
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    return Err(e);
                }
            }
            i += 1;
        }
        log_info(i, self);
        let render_hash = reader.final_hash();
        self.signature = SHA1::from_stream(&mut reader).unwrap();

        if render_hash != self.signature {
            return Err(GitError::InvalidPackFile(format!(
                "The pack file hash {} does not match the trailer hash {}",
                render_hash, self.signature
            )));
        }

        let end = utils::is_eof(&mut reader);
        if !end {
            return Err(GitError::InvalidPackFile(
                "The pack file is not at the end".to_string(),
            ));
        }

        self.pool.join(); // wait for all threads to finish
                          // !Attention: Caches threadpool may not stop, but it's not a problem (garbage file data)
                          // So that files != self.number
        assert_eq!(self.waitlist.map_offset.len(), 0);
        assert_eq!(self.waitlist.map_ref.len(), 0);
        assert_eq!(self.number, caches.total_inserted());
        tracing::info!(
            "The pack file has been decoded successfully, takes: [ {:?} ]",
            time.elapsed()
        );
        self.caches.clear(); // clear cached objects & stop threads
        assert_eq!(self.cache_objs_mem_used(), 0); // all the objs should be dropped until here

        // impl in Drop Trait
        // if self.clean_tmp {
        //     self.caches.remove_tmp_dir();
        // }

        Ok(())
    }

    /// Decode a Pack in a new thread and send the CacheObjects while decoding.
    /// <br> Attention: It will consume the `pack` and return in a JoinHandle.
    pub fn decode_async(
        mut self,
        mut pack: (impl BufRead + Send + 'static),
        sender: UnboundedSender<Entry>,
    ) -> JoinHandle<Pack> {
        thread::spawn(move || {
            self.decode(&mut pack, move |entry, _| {
                if let Err(e) = sender.send(entry) {
                    eprintln!("Channel full, failed to send entry: {:?}", e);
                }
            })
            .unwrap();
            self
        })
    }

    /// Decodes a `Pack` from a `Stream` of `Bytes`, and sends the `Entry` while decoding.
    pub async fn decode_stream(
        mut self,
        mut stream: impl Stream<Item = Result<Bytes, Error>> + Unpin + Send + 'static,
        sender: UnboundedSender<Entry>,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut reader = StreamBufReader::new(rx);
        tokio::spawn(async move {
            while let Some(chunk) = stream.next().await {
                let data = chunk.unwrap().to_vec();
                if let Err(e) = tx.send(data) {
                    eprintln!("Sending Error: {:?}", e);
                    break;
                }
            }
        });
        // CPU-bound task, so use spawn_blocking
        // DO NOT use thread::spawn, because it will block tokio runtime (if single-threaded runtime, like in tests)
        tokio::task::spawn_blocking(move || {
            self.decode(&mut reader, move |entry: Entry, _| {
                // as we used unbound channel here, it will never full so can be send with synchronous
                if let Err(e) = sender.send(entry) {
                    eprintln!("Channel full, failed to send entry: {:?}", e);
                }
            })
            .unwrap();
            self
        })
        .await
        .unwrap()
    }

    /// CacheObjects + Index size of Caches
    fn memory_used(&self) -> usize {
        self.cache_objs_mem_used() + self.caches.memory_used_index()
    }

    /// The total memory used by the CacheObjects of this Pack
    fn cache_objs_mem_used(&self) -> usize {
        self.cache_objs_mem.load(Ordering::Acquire)
    }

    /// Rebuild the Delta Object in a new thread & process the objects waiting for it recursively.
    /// <br> This function must be *static*, because [&self] can't be moved into a new thread.
    fn process_delta(
        shared_params: Arc<SharedParams>,
        delta_obj: CacheObject,
        base_obj: Arc<CacheObject>,
    ) {
        shared_params.pool.clone().execute(move || {
            let mut new_obj = Pack::rebuild_delta(delta_obj, base_obj);
            new_obj.set_mem_recorder(shared_params.cache_objs_mem_size.clone());
            new_obj.record_mem_size();
            Self::cache_obj_and_process_waitlist(shared_params, new_obj); //Indirect Recursion
        });
    }

    /// Cache the new object & process the objects waiting for it (in multi-threading).
    fn cache_obj_and_process_waitlist(shared_params: Arc<SharedParams>, new_obj: CacheObject) {
        (shared_params.callback)(new_obj.to_entry(), new_obj.offset);
        let new_obj = shared_params.caches.insert(
            new_obj.offset,
            new_obj.base_object_hash().unwrap(),
            new_obj,
        );
        Self::process_waitlist(shared_params, new_obj);
    }

    fn process_waitlist(shared_params: Arc<SharedParams>, base_obj: Arc<CacheObject>) {
        let wait_objs = shared_params
            .waitlist
            .take(base_obj.offset, base_obj.base_object_hash().unwrap());
        for obj in wait_objs {
            // Process the objects waiting for the new object(base_obj = new_obj)
            Self::process_delta(shared_params.clone(), obj, base_obj.clone());
        }
    }

    /// Reconstruct the Delta Object based on the "base object"
    /// and return the new object.
    pub fn rebuild_delta(delta_obj: CacheObject, base_obj: Arc<CacheObject>) -> CacheObject {
        const COPY_INSTRUCTION_FLAG: u8 = 1 << 7;
        const COPY_OFFSET_BYTES: u8 = 4;
        const COPY_SIZE_BYTES: u8 = 3;
        const COPY_ZERO_SIZE: usize = 0x10000;

        let mut stream = Cursor::new(&delta_obj.data_decompressed);

        // Read the base object size
        // (Size Encoding)
        let (base_size, result_size) = utils::read_delta_object_size(&mut stream).unwrap();

        // Get the base object data
        let base_info = &base_obj.data_decompressed;
        assert_eq!(base_info.len(), base_size, "Base object size mismatch");

        let mut result = Vec::with_capacity(result_size);

        loop {
            // Check if the stream has ended, meaning the new object is done
            let instruction = match utils::read_bytes(&mut stream) {
                Ok([instruction]) => instruction,
                Err(err) if err.kind() == ErrorKind::UnexpectedEof => break,
                Err(err) => {
                    panic!(
                        "{}",
                        GitError::DeltaObjectError(format!("Wrong instruction in delta :{}", err))
                    );
                }
            };

            if instruction & COPY_INSTRUCTION_FLAG == 0 {
                // Data instruction; the instruction byte specifies the number of data bytes
                if instruction == 0 {
                    // Appending 0 bytes doesn't make sense, so git disallows it
                    panic!(
                        "{}",
                        GitError::DeltaObjectError(String::from("Invalid data instruction"))
                    );
                }

                // Append the provided bytes
                let mut data = vec![0; instruction as usize];
                stream.read_exact(&mut data).unwrap();
                result.extend_from_slice(&data);
            } else {
                // Copy instruction
                // +----------+---------+---------+---------+---------+-------+-------+-------+
                // | 1xxxxxxx | offset1 | offset2 | offset3 | offset4 | size1 | size2 | size3 |
                // +----------+---------+---------+---------+---------+-------+-------+-------+
                let mut nonzero_bytes = instruction;
                let offset =
                    utils::read_partial_int(&mut stream, COPY_OFFSET_BYTES, &mut nonzero_bytes)
                        .unwrap();
                let mut size =
                    utils::read_partial_int(&mut stream, COPY_SIZE_BYTES, &mut nonzero_bytes)
                        .unwrap();
                if size == 0 {
                    // Copying 0 bytes doesn't make sense, so git assumes a different size
                    size = COPY_ZERO_SIZE;
                }
                // Copy bytes from the base object
                let base_data = base_info.get(offset..(offset + size)).ok_or_else(|| {
                    GitError::DeltaObjectError("Invalid copy instruction".to_string())
                });

                match base_data {
                    Ok(data) => result.extend_from_slice(data),
                    Err(e) => panic!("{}", e),
                }
            }
        }
        assert_eq!(result_size, result.len(), "Result size mismatch");

        let hash = utils::calculate_object_hash(base_obj.object_type(), &result);
        // create new obj from `delta_obj` & `result` instead of modifying `delta_obj` for heap-size recording
        CacheObject {
            info: CacheObjectInfo::BaseObject(base_obj.object_type(), hash),
            offset: delta_obj.offset,
            data_decompressed: result,
            mem_recorder: None,
        } // Canonical form (Complete Object)
          // Memory recording will happen after this function returns. See `process_delta`
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::{env, path::PathBuf};

    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use tokio_util::io::ReaderStream;

    use crate::internal::pack::tests::init_logger;
    use crate::internal::pack::Pack;
    use futures_util::TryStreamExt;

    #[test]
    fn test_pack_check_header() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let f = fs::File::open(source).unwrap();
        let mut buf_reader = BufReader::new(f);
        let (object_num, _) = Pack::check_header(&mut buf_reader).unwrap();

        assert_eq!(object_num, 358109);
    }

    #[test]
    fn test_decompress_data() {
        let data = b"Hello, world!"; // Sample data to compress and then decompress
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let compressed_data = encoder.finish().unwrap();
        let compressed_size = compressed_data.len();

        // Create a cursor for the compressed data to simulate a BufRead source
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(compressed_data);
        let expected_size = data.len();

        // Decompress the data and assert correctness
        let mut p = Pack::new(None, None, None, true);
        let result = p.decompress_data(&mut cursor, expected_size);
        match result {
            Ok((decompressed_data, bytes_read)) => {
                assert_eq!(bytes_read, compressed_size);
                assert_eq!(decompressed_data, data);
            }
            Err(e) => panic!("Decompression failed: {:?}", e),
        }
    }

    #[test]
    fn test_pack_decode_without_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new(None, Some(1024 * 1024 * 20), Some(tmp), true);
        p.decode(&mut buffered, |_, _| {}).unwrap();
    }

    #[test]
    // #[traced_test]
    fn test_pack_decode_with_ref_delta() {
        init_logger();

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/ref-delta-65d47638aa7cb7c39f1bd1d5011a415439b887a8.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new(None, Some(1024 * 1024 * 20), Some(tmp), true);
        p.decode(&mut buffered, |_, _| {}).unwrap();
    }

    #[test]
    fn test_pack_decode_no_mem_limit() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new(None, None, Some(tmp), true);
        p.decode(&mut buffered, |_, _| {}).unwrap();
    }

    #[test]
    #[ignore] // Take too long time
    fn test_pack_decode_with_large_file_with_delta_without_ref() {
        init_logger();

        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new(
            Some(20),
            Some(1024 * 1024 * 1024 * 2),
            Some(tmp.clone()),
            true,
        );
        let rt = p.decode(&mut buffered, |_obj, _offset| {
            // println!("{:?} {}", obj.hash.to_string(), offset);
        });
        if let Err(e) = rt {
            fs::remove_dir_all(tmp).unwrap();
            panic!("Error: {:?}", e);
        }
    } // it will be stuck on dropping `Pack` on Windows if `mem_size` is None, so we need `mimalloc`

    #[tokio::test]
    async fn test_decode_large_file_stream() {
        init_logger();
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");
        let f = tokio::fs::File::open(source).await.unwrap();
        let stream = ReaderStream::new(f).map_err(axum::Error::new);
        let p = Pack::new(
            Some(20),
            Some(1024 * 1024 * 1024 * 4),
            Some(tmp.clone()),
            true,
        );

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = tokio::spawn(async move { p.decode_stream(stream, tx).await });
        let count = Arc::new(AtomicUsize::new(0));
        let count_c = count.clone();
        // in tests, RUNTIME is single-threaded, so `sync code` will block the tokio runtime
        tokio::task::spawn_blocking(move || {
            let mut cnt = 0;
            while let Ok(_entry) = rx.try_recv() {
                cnt += 1; //use entry here
            }
            tracing::info!("Received: {}", cnt);
            count_c.store(cnt, Ordering::Release);
        })
        .await
        .unwrap();
        let p = handle.await.unwrap();
        assert_eq!(count.load(Ordering::Acquire), p.number);
    }

    #[test]
    #[ignore] // Take too long time, duplicate with `test_decode_large_file_stream`
    fn test_decode_large_file_async() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");
        let f = fs::File::open(source).unwrap();
        let buffered = BufReader::new(f);
        let p = Pack::new(
            Some(20),
            Some(1024 * 1024 * 1024 * 2),
            Some(tmp.clone()),
            true,
        );

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = p.decode_async(buffered, tx); // new thread
        let mut cnt = 0;
        while let Ok(_entry) = rx.try_recv() {
            cnt += 1; //use entry here
        }
        let p = handle.join().unwrap();
        assert_eq!(cnt, p.number);
    }

    #[test]
    fn test_pack_decode_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new(None, Some(1024 * 1024 * 20), Some(tmp), true);
        p.decode(&mut buffered, |_, _| {}).unwrap();
    }

    #[test]
    #[ignore] // Take too long time
    fn test_pack_decode_multi_task_with_large_file_with_delta_without_ref() {
        let task1 = std::thread::spawn(|| {
            test_pack_decode_with_large_file_with_delta_without_ref();
        });
        let task2 = std::thread::spawn(|| {
            test_pack_decode_with_large_file_with_delta_without_ref();
        });

        task1.join().unwrap();
        task2.join().unwrap();
    }
}
