//!
//!
//!
//!
//!
//!
use std::io::{self, BufRead, Cursor, ErrorKind, Read, Seek};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use flate2::bufread::ZlibDecoder;
use threadpool::ThreadPool;

use venus::errors::GitError;
use venus::hash::SHA1;
use venus::internal::object::types::ObjectType;

use crate::internal::pack::cache_object::CacheObject;
use crate::internal::pack::cache::Caches;
use crate::internal::pack::waitlist::Waitlist;
use crate::internal::pack::wrapper::Wrapper;
use crate::internal::pack::{utils, Pack};
use uuid::Uuid;
use super::cache::_Cache;

impl Default for Pack {
    fn default() -> Self {
        Self::new(num_cpus::get())
    }
}
impl Pack {
    /// @param thread_num: The number of threads to use for decoding and cache
    /// for example, thread_num = 4 will use up to 8 threads (4 for decoding and 4 for cache)
    pub fn new(thread_num: usize) -> Self {
        Pack {
            number: 0,
            signature: SHA1::default(),
            objects: Vec::new(),
            pool: Arc::new(ThreadPool::new(thread_num)),
            waitlist: Arc::new(Waitlist::new(0)),
        }
    }

    /// Checks and reads the header of a Git pack file.
    ///
    /// This function reads the first 12 bytes of a pack file, which include the "PACK" magic identifier,
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
    /// * `Err(GitError)`: On failure, returns a `GitError` with a description of the issue.
    ///
    /// # Errors
    /// This function can return an error in the following situations:
    /// * If the pack file does not start with the "PACK" magic identifier.
    /// * If the pack file's version number is not 2.
    /// * If there are any issues reading from the provided `pack` source.
    pub fn check_header(pack: &mut (impl Read + BufRead)) -> Result<(u32, Vec<u8>), GitError> {
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
            },
            Err(_e) => {
                // If there is an error in reading, return a GitError
                return Err(GitError::InvalidPackHeader(format!(
                    "{},{},{},{}",
                    magic[0], magic[1], magic[2], magic[3]
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
                // If read is successful, proceed
            },
            Err(_e) => {
                // If there is an error in reading, return a GitError
                return Err(GitError::InvalidPackHeader(format!(
                    "{},{},{},{}",
                    version_bytes[0], version_bytes[1], version_bytes[2], version_bytes[3]
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
            },
            Err(_e) => {
                // If there is an error in reading, return a GitError
                Err(GitError::InvalidPackHeader(format!(
                    "{},{},{},{}",
                    object_num_bytes[0], object_num_bytes[1], object_num_bytes[2], object_num_bytes[3]
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
    /// * A tuple with a `Vec<u8>` of decompressed data, a `Vec<u8>` of the original compressed data,
    ///   and the total number of input bytes processed,
    /// * Or a `GitError` in case of a mismatch in expected size or any other reading error.
    ///
    pub fn decompress_data(&mut self, pack: &mut (impl Read + BufRead + Send), expected_size: usize, ) -> Result<(Vec<u8>, usize), GitError> {
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
            },
            Err(e) => {
                // If there is an error in reading, return a GitError
                Err(GitError::InvalidPackFile(format!( "Decompression error: {}", e)))
            }
        }
    }

    /// Decodes a pack object from a given Read and BufRead source and returns the original compressed data.
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
    pub fn decode_pack_object(&mut self, pack: &mut (impl Read + BufRead + Send), offset: &mut usize) -> Result<CacheObject, GitError> {
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
            },
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

                Ok(CacheObject {
                    base_offset,
                    base_ref: SHA1::default(),
                    data_decompress: data,
                    obj_type: t,
                    offset: init_offset,
                    hash: SHA1::default(),
                })
            },
            ObjectType::HashDelta => {
                // Read 20 bytes to get the reference object SHA1 hash
                let mut buf_ref = [0; 20];
                pack.read_exact(&mut buf_ref).unwrap();
                let ref_sha1 = SHA1::from_bytes(buf_ref.as_ref()); //TODO SHA1::from_stream()
                // Offset is incremented by 20 bytes
                *offset += 20; //TODO 改为常量

                let (data, raw_size) = self.decompress_data(pack, size)?;
                *offset += raw_size;

                Ok(CacheObject {
                    base_offset: 0,
                    base_ref: ref_sha1,
                    data_decompress: data,
                    obj_type: t,
                    offset: init_offset,
                    hash: SHA1::default(),
                })
            }
        }
    }

    /// Decodes a pack file from a given Read and BufRead source and get a vec of objects.
    ///
    ///
    pub fn decode(&mut self, pack: &mut (impl Read + BufRead + Seek + Send), mem_size: usize, tmp_path: PathBuf) -> Result<(), GitError> {

        #[cfg(debug_assertions)]
        let time = Instant::now();
        
        // use random subdirectory to avoid conflicts with other files
        let tmp_path = tmp_path.join(Uuid::new_v4().to_string()); //maybe Snowflake or ULID is better (less collision)
        let caches = Arc::new(Caches::new(Some(mem_size), Some(tmp_path.clone()), self.pool.max_count()));
        
        let mut reader = Wrapper::new(io::BufReader::new(pack));

        let result = Pack::check_header(&mut reader);
        match result {
            Ok((object_num, _)) => {
                self.number = object_num as usize;
            },
            Err(e) => {
                return Err(e);
            }
        }
        println!("The pack file has {} objects", self.number);

        let mut offset: usize = 12;
        let mut i = 1;

        while i <= self.number {
            #[cfg(debug_assertions)]
            {
                if i % 10000 == 0 {
                    println!("excute {:?} \t objects decoded: {}, \t decode queue: {} \t cache queue: {}",time.elapsed(), i, self.pool.queued_count(), caches.queued_tasks());
                }
            }
            while self.pool.queued_count() > 10000 { // TODO: replace with memory related condition
                std::thread::sleep(std::time::Duration::from_millis(100));  
            }
            let r: Result<CacheObject, GitError> = self.decode_pack_object(&mut reader, &mut offset);
            match r {
                Ok(obj) => {
                    let caches = caches.clone();
                    let pool = self.pool.clone();
                    let waitlist = self.waitlist.clone();
                    self.pool.execute(move || {
                        match obj.obj_type {
                            ObjectType::Commit | ObjectType::Tree | ObjectType::Blob | ObjectType::Tag => {
                                Pack::cache_obj_and_process_waitlist(pool, waitlist, caches, obj);
                            },
                            ObjectType::OffsetDelta => {
                                if let Some(base_obj) = caches.get_by_offset(obj.base_offset) {
                                    Self::process_delta(pool, waitlist, caches, obj, base_obj);
                                } else {
                                    // You can delete this 'if' block ↑, because there are Second check in 'else'
                                    // It will be more readable, but the performance will be slightly reduced
                                    let base_offset = obj.base_offset;
                                    waitlist.insert_offset(obj.base_offset, obj);
                                    // Second check: prevent that the base_obj thread has finished before the waitlist insert
                                    if let Some(base_obj) = caches.get_by_offset(base_offset) {
                                        Pack::process_waitlist(pool, waitlist, caches, base_obj);
                                    }
                                }
                            },
                            ObjectType::HashDelta => {
                                if let Some(base_obj) = caches.get_by_hash(obj.base_ref) {
                                    Self::process_delta(pool, waitlist, caches, obj, base_obj);
                                } else {
                                    let base_ref = obj.base_ref;
                                    waitlist.insert_ref(obj.base_ref, obj);
                                    if let Some(base_obj) = caches.get_by_hash(base_ref) {
                                        Pack::process_waitlist(pool, waitlist, caches, base_obj);
                                    }
                                }
                            }
                        }
                    });
                },
                Err(e) => {
                    return Err(e);
                }
            }

            i += 1;
        }

        let render_hash = reader.final_hash();
        let mut trailer_buf = [0; 20];
        reader.read_exact(&mut trailer_buf).unwrap();
        self.signature = SHA1::from_bytes(trailer_buf.as_ref());

        if render_hash != self.signature {
            return Err(GitError::InvalidPackFile(format!(
                "The pack file hash {} does not match the trailer hash {}",
                render_hash.to_plain_str(),
                self.signature.to_plain_str()
            )));
        }

        let end = utils::is_eof(&mut reader);
        if !end {
            return Err(GitError::InvalidPackFile(
                "The pack file is not at the end".to_string()
            ));
        }

        self.pool.join(); // wait for all threads to finish
        // !Attention: Caches threadpool may not stop, but it's not a problem (garbage file data)
        // So that files != self.number
        println!("The pack file has been decoded successfully");
        assert_eq!(self.waitlist.map_offset.len(), 0);
        assert_eq!(self.waitlist.map_ref.len(), 0);
        assert_eq!(self.number, caches.total_inserted());

        // todo: difficult to stop threads in cache, so we didn't remove the temp file temporarily
        drop(caches);
        // fs::remove_dir_all(tmp_path).unwrap();

        Ok(())
    }

    /// Rebuild the Delta Object in a new thread & process the objects waiting for it recursively.
    /// <br> This function must be *static*, because [&self] can't be moved into a new thread.
    fn process_delta(pool: Arc<ThreadPool>, waitlist: Arc<Waitlist>, caches: Arc<Caches>, delta_obj: CacheObject, base_obj: Arc<CacheObject>) {
        pool.clone().execute(move || {
            let new_obj = Pack::rebuild_delta(delta_obj, base_obj);
            Pack::cache_obj_and_process_waitlist(pool, waitlist, caches, new_obj); //Indirect Recursion
        });
    }

    /// Cache the new object & process the objects waiting for it (in multi-threading).
    fn cache_obj_and_process_waitlist(pool: Arc<ThreadPool>, waitlist: Arc<Waitlist>, caches: Arc<Caches>, new_obj: CacheObject) {
        let new_obj = caches.insert(new_obj.offset, new_obj.hash, new_obj);
        Pack::process_waitlist(pool, waitlist, caches, new_obj);
    }

    fn process_waitlist(pool: Arc<ThreadPool>, waitlist: Arc<Waitlist>, caches: Arc<Caches>, base_obj: Arc<CacheObject>) {
        let wait_objs = waitlist.take(base_obj.offset, base_obj.hash);
        for obj in wait_objs {
            // Process the objects waiting for the new object(base_obj = new_obj)
            Pack::process_delta(pool.clone(), waitlist.clone(), caches.clone(), obj, base_obj.clone());
        }
    }

    /// Reconstruct the Delta Object based on the "base object"
    fn rebuild_delta(mut delta_obj: CacheObject, base_obj: Arc<CacheObject>) -> CacheObject {
        const COPY_INSTRUCTION_FLAG: u8 = 1 << 7;
        const COPY_OFFSET_BYTES: u8 = 4;
        const COPY_SIZE_BYTES: u8 = 3;
        const COPY_ZERO_SIZE: usize = 0x10000;

        let mut stream = Cursor::new(delta_obj.data_decompress);

        // Read the base object size & Result Size
        // (Size Encoding)
        let (base_size, _) = utils::read_varint_le(&mut stream).unwrap();
        let (result_size, _) = utils::read_varint_le(&mut stream).unwrap();

        //Get the base object row data
        let base_info = &base_obj.data_decompress;
        assert_eq!(base_info.len() as u64, base_size);

        let mut result = Vec::with_capacity(result_size as usize);

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
                let offset = utils::read_partial_int(&mut stream, COPY_OFFSET_BYTES, &mut nonzero_bytes).unwrap();
                let mut size = utils::read_partial_int(&mut stream, COPY_SIZE_BYTES, &mut nonzero_bytes).unwrap();
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

        delta_obj.data_decompress = result;
        delta_obj.obj_type = base_obj.obj_type; // Same as the Type of base object
        delta_obj.hash = utils::calculate_object_hash(delta_obj.obj_type, &delta_obj.data_decompress);

        delta_obj //Canonical form (Complete Object)
    }
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::io::Cursor;
    use std::time::Instant;
    use std::{env, path::PathBuf};

    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    use crate::internal::pack::Pack;

    #[test]
    fn test_pack_check_header() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let f = std::fs::File::open(source).unwrap();
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

        // Create a cursor for the compressed data to simulate a Read + BufRead source
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(compressed_data);
        let expected_size = data.len();

        // Decompress the data and assert correctness
        let mut p = Pack::default();
        let result = p.decompress_data(&mut cursor, expected_size);
        match result {
            Ok((decompressed_data, bytes_read)) => {
                assert_eq!(bytes_read, compressed_size);
                assert_eq!(decompressed_data, data);
            },
            Err(e) => panic!("Decompression failed: {:?}", e),
        }
    }

    #[test]
    fn test_pack_decode_without_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::default();
        p.decode(&mut buffered, 0, tmp).unwrap();
    }

    #[test]
    fn test_pack_decode_with_ref_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/ref-delta-65d47638aa7cb7c39f1bd1d5011a415439b887a8.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::default();
        p.decode(&mut buffered, 0, tmp).unwrap();
    }

    #[test]
    fn test_pack_decode_with_large_file_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        // let mut p = Pack::default(); //Pack::new(2);
        let mut p = Pack::new(20);
        let start = Instant::now();
        p.decode(&mut buffered, 0, tmp).unwrap();
        println!("Test took {:?}", start.elapsed());
    }

    #[test]
    fn test_pack_decode_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack");

        let tmp = PathBuf::from("/tmp/.cache_temp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::default();
        p.decode(&mut buffered, 1024*1024*20, tmp).unwrap();
    }
}
