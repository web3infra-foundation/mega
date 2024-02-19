
//!
//! 
//! 
//! 
//! 
//! 
use std::io::{self, Read, BufRead, Seek, ErrorKind, Cursor};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use dashmap::DashMap;

use flate2::bufread::ZlibDecoder;
use threadpool::ThreadPool;

use venus::errors::GitError;
use venus::hash::SHA1;
use venus::internal::object::types::ObjectType;

use crate::internal::pack::Pack;
use crate::internal::pack::{Pack, utils};
use crate::internal::pack::wrapper::Wrapper;
use crate::internal::pack::utils::{read_offset_encoding, read_type_and_varint_size};
use crate::internal::pack::utils::read_varint_le;
use crate::internal::pack::utils::is_eof;
use crate::internal::pack::cache::CacheObject;
use crate::internal::pack::cache::Caches;

impl Pack {
    pub fn new() -> Self {
        Pack {
            number: 0,
            signature: SHA1::default(),
            objects: Vec::new(),
            pool: ThreadPool::new(num_cpus::get()),
            waitlist_offset: Arc::new(DashMap::new()),
            waitlist_ref: Arc::new(DashMap::new()),
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
    pub fn decompress_data(&mut self, pack: &mut (impl Read + BufRead + Send), expected_size: usize) -> Result<(Vec<u8>, usize), GitError> {        
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
                }
            },
            Err(e) => {
                // If there is an error in reading, return a GitError
                Err(GitError::InvalidPackFile(format!("Decompression error: {}", e)))
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
        // Attempt to read the type and size, handle potential errors
        let (type_bits, size) = match read_type_and_varint_size(pack, offset) {
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
                let (data, object_offset) = self.decompress_data(pack, size)?;
                *offset += object_offset;

                Ok(CacheObject {
                    base_offset: 0,
                    base_ref: SHA1::default(),
                    data_decompress: data,
                    object_type: t,
                    offset: *offset,
                    hash: SHA1::default(),
                })
            },
            ObjectType::OffsetDelta => {
                let (_base_offset, step_offset) = read_offset_encoding(pack).unwrap();
                *offset += step_offset;
                
                let (data, object_offset) = self.decompress_data(pack, size)?;
                *offset += object_offset;

                Ok(CacheObject {
                    base_offset: object_offset,
                    base_ref: SHA1::default(),
                    data_decompress: data,
                    object_type: t,
                    offset: *offset,
                    hash: SHA1::default(),
                })
            },
            ObjectType::HashDelta => {
                // Read 20 bytes to get the reference object SHA1 hash
                let mut buf_ref = [0; 20];
                pack.read_exact(&mut buf_ref).unwrap();
                let ref_sha1 = SHA1::from_bytes(buf_ref.as_ref());
                // Offset is incremented by 20 bytes
                *offset += 20;

                let (data, object_offset) = self.decompress_data(pack, size)?;
                *offset += object_offset;

                Ok(CacheObject {
                    base_offset: 0,
                    base_ref: ref_sha1,
                    data_decompress: data,
                    object_type: t,
                    offset: *offset,
                    hash: SHA1::default(),
                })
            }
        }
    }


    /// Decodes a pack file from a given Read and BufRead source and get a vec of objects.
    /// 
    /// 
    pub fn decode(&mut self, pack: &mut (impl Read + BufRead + Seek + Send), mem_size: usize, tmp_path: PathBuf) -> Result<(), GitError> {
        let caches = Caches {
            map_offset: HashMap::new(),
            map_hash: HashMap::new(),
            mem_size,
            tmp_path,
        };
        let caches = Arc::new(RwLock::new(caches));

        let mut render = Wrapper::new(io::BufReader::new(pack));

        let result = Pack::check_header(&mut render);
        match result {
            Ok((object_num, _)) => {
                self.number = object_num as usize;
            },
            Err(e) => {
                return Err(e);
            }
        }
        
        let mut offset: usize = 12;
        let mut i = 1;

        while i <= self.number {
            let r: Result<CacheObject, GitError> = self.decode_pack_object(&mut render, &mut offset);
            match r {
                Ok(obj) => {
                    // caches.insert(cache.offset, cache); //TODO 根据类型 分类处理

                    match obj.object_type {
                        ObjectType::Commit | ObjectType::Tree | ObjectType::Blob | ObjectType::Tag => {
                            let mut caches = caches.write().unwrap();
                            caches.insert(obj.offset, obj); //insert()
                        },
                        ObjectType::OffsetDelta | ObjectType::HashDelta => {
                            let mut base_obj: Option<Arc<CacheObject>> = None;
                            if matches!(obj.object_type, ObjectType::OffsetDelta) {
                                let caches = caches.read().unwrap();
                                match caches.map_offset.get(&obj.base_offset) {  // get_by_offset()
                                    None => { //TODO 需要minus计算
                                        self.waitlist_offset.entry(obj.base_offset).or_insert(Vec::new()).push(obj);
                                    }
                                    Some(hash) => {
                                        base_obj = Some(caches.map_hash.get(hash).unwrap().clone()); //map_offset存在 map_hash一定存在
                                    }
                                }
                            } else {
                                let caches = caches.read().unwrap();
                                match caches.map_hash.get(&obj.base_ref) { // get_by_hash()
                                    None => {
                                        self.waitlist_ref.entry(obj.base_ref).or_insert(Vec::new()).push(obj);
                                    }
                                    Some(cache_obj) => {
                                        base_obj = Some(cache_obj.clone());
                                    }
                                }
                            }
                            if let Some(base_obj) = base_obj {
                                self.process_delta(caches.clone(), obj.clone(), base_obj); //TODO 理论上不需要clone
                            }
                        }
                    }

                },
                Err(e) => {
                    return Err(e);
                }
            }

            i += 1;
        }

        let render_hash = render.final_hash();
        let mut tailer_buf= [0; 20];
        render.read_exact(&mut tailer_buf).unwrap();
        self.signature = SHA1::from_bytes(tailer_buf.as_ref());

        if render_hash != self.signature {
            return Err(GitError::InvalidPackFile(format!(
                "The pack file hash {} does not match the tailer hash {}",
                render_hash.to_plain_str(),
                self.signature.to_plain_str()
            )));
        }

        let end = is_eof(&mut render);
        if !end {
            return Err(GitError::InvalidPackFile(
                "The pack file is not at the end".to_string()
            ));
        }

        self.pool.join(); // wait for all threads to finish
        Ok(())
    }

    fn process_delta(&self, caches: Arc<RwLock<Caches>>, delta_obj: CacheObject, base_obj: Arc<CacheObject>) {
        let waitlist_offset = Arc::clone(&self.waitlist_offset);
        let waitlist_ref = Arc::clone(&self.waitlist_ref);
        self.pool.execute(move || {
            let new_obj = Pack::rebuild_delta(delta_obj, base_obj);
            let new_hash = new_obj.hash;

            caches.write().unwrap().insert(new_obj.offset, new_obj);
            let new_obj = caches.read().unwrap().map_hash.get(&new_hash).unwrap().clone();

            let mut waitlist = Vec::new();
            if let Some((_, vec)) = waitlist_offset.remove(&new_obj.offset) {
                waitlist.extend(vec);
            }
            if let Some((_, vec)) = waitlist_ref.remove(&new_obj.hash) {
                waitlist.extend(vec);
            }

            for obj in waitlist {
                self.process_delta(caches.clone(), obj, new_obj.clone()); // TODO
            }

        });
    }

    /// Reconstruct the Delta Object based on the "base object"
    ///
    fn rebuild_delta(mut delta_object: CacheObject, base_object: Arc<CacheObject>) -> CacheObject {
        const COPY_INSTRUCTION_FLAG: u8 = 1 << 7;
        const COPY_OFFSET_BYTES: u8 = 4;
        const COPY_SIZE_BYTES: u8 = 3;
        const COPY_ZERO_SIZE: usize = 0x10000;

        let mut stream = Cursor::new(delta_object.data_decompress);

        // Read the base object size & Result Size
        // (Size Encoding)
        let (base_size, _) = read_varint_le(&mut stream).unwrap();
        let (result_size, _) = read_varint_le(&mut stream).unwrap();

        //Get the base object row data
        let base_info = &base_object.data_decompress;
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
                let offset =
                    utils::read_partial_int(&mut stream, COPY_OFFSET_BYTES, &mut nonzero_bytes).unwrap();
                let mut size =
                    utils::read_partial_int(&mut stream, COPY_SIZE_BYTES, &mut nonzero_bytes).unwrap();
                if size == 0 {
                    // Copying 0 bytes doesn't make sense, so git assumes a different size
                    size = COPY_ZERO_SIZE;
                }
                // Copy bytes from the base object
                let base_data = base_info
                    .get(offset..(offset + size))
                    .ok_or_else(|| GitError::DeltaObjectError("Invalid copy instruction".to_string()));

                match base_data {
                    Ok(data) => result.extend_from_slice(data),
                    Err(e) => panic!("{}", e),
                }
            }
        }

        delta_object.data_decompress = result;
        delta_object.object_type = base_object.object_type; // Same as the Type of base object
        delta_object.hash = SHA1::default(); //TODO 计算hash

        return delta_object; //Canonical form
    }

}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, env};
    use std::io::Cursor;
    use std::io::BufReader;
    use std::io::prelude::*;
    

    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    
    use venus::hash::SHA1;
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

        // Create a cursor for the compressed data to simulate a Read + BufRead source
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(compressed_data);
        let expected_size = data.len();

        // Decompress the data and assert correctness
        let mut p = Pack::new();
        let result = p.decompress_data(&mut cursor, expected_size);
        match result {
            Ok((decompressed_data, _)) => {
                assert_eq!(decompressed_data, data);
            },
            Err(e) => panic!("Decompression failed: {:?}", e),
        }
    }

    #[test]
    fn test_pack_decode_without_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack");

        let tmp = PathBuf::from("/tmp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new();
        p.decode(&mut buffered, 0, tmp).unwrap();
    }

    #[test]
    fn test_pack_decode_with_ref_delta() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/ref-delta-65d47638aa7cb7c39f1bd1d5011a415439b887a8.pack");

        let tmp = PathBuf::from("/tmp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new();
        p.decode(&mut buffered, 0, tmp).unwrap();
    }

    #[test]
    fn test_pack_decode_with_large_file_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/git-2d187177923cd618a75da6c6db45bb89d92bd504.pack");

        let tmp = PathBuf::from("/tmp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new();
        p.decode(&mut buffered, 0, tmp).unwrap();
    }

    #[test]
    fn test_pack_decode_with_delta_without_ref() {
        let mut source = PathBuf::from(env::current_dir().unwrap().parent().unwrap());
        source.push("tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack");

        let tmp = PathBuf::from("/tmp");

        let f = std::fs::File::open(source).unwrap();
        let mut buffered = BufReader::new(f);
        let mut p = Pack::new();
        p.decode(&mut buffered, 0, tmp).unwrap();
    }
}