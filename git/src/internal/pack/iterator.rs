use crate::hash::Hash;
use crate::{
    internal::{
        object::{blob::Blob, commit::Commit, tag::Tag, tree::Tree},
        pack::delta::DeltaReader,
        zlib::stream::inflate::ReadBoxed,
        GitError, ObjectType,
    },
    utils,
};

use std::{io::BufRead, sync::Arc};

use crate::internal::object::{cache::ObjectCache, ObjectT};
type IteratorResult = Result<Arc<dyn ObjectT>, GitError>;

///
pub struct EntriesIter<BR> {
    inner: BR,
    offset: usize,
    objects_left: u32,
    cache: ObjectCache,
}

impl<BR: std::io::BufRead> EntriesIter<BR> {
    //After Pack::check_header
    pub fn new(r: BR, obj_num: u32) -> Self {
        Self {
            inner: r,
            offset: 12,
            objects_left: obj_num,
            cache: ObjectCache::new(),
        }
    }

    pub async fn next_obj(&mut self) -> IteratorResult {
        self.objects_left -= 1;
        let mut iter_offset: usize = 0;
        // Read the Object Type and Total Size of one Object
        let (type_num, size) = utils::read_type_and_size(&mut self.inner).unwrap();
        //Get the Object according to the Types Enum
        let obj_type = ObjectType::number2type(type_num).unwrap();
        iter_offset += utils::get_7bit_count(size << 3);

        let obj = if (1..=4).contains(&type_num) {
            read_object(&mut self.inner, obj_type, size, &mut iter_offset).await?
        } else {
            let base_object: Arc<dyn ObjectT>;

            if type_num == 6 {
                // Offset Delta Object
                let offset = self.offset;
                let delta_offset = utils::read_offset_encoding(&mut self.inner).unwrap() as usize;
                iter_offset += utils::get_7bit_count(delta_offset);
                // Count the base object offset and get the base object from the cache in EntriesIter
                let base_offset = offset
                    .checked_sub(delta_offset)
                    .ok_or_else(|| {
                        GitError::InvalidObjectInfo("Invalid OffsetDelta offset".to_string())
                    })
                    .unwrap();

                base_object = self
                    .cache
                    .get(base_offset)
                    .ok_or_else(|| {
                        println!("wrong base offset :{}", base_offset);
                        GitError::DeltaObjectError("cant' find base obj from offset".to_string())
                    })
                    .unwrap();
            } else if type_num == 7 {
                // Ref Delta Object
                let hash = utils::read_hash(&mut self.inner).unwrap();
                iter_offset += 20;
                base_object = self
                    .cache
                    .get_hash(hash)
                    .ok_or_else(|| {
                        GitError::DeltaObjectError(
                            "cant' find base obj from hash value ".to_string(),
                        )
                    })
                    .unwrap();
            } else {
                return Err(ObjectType::number2type(type_num).err().unwrap());
            }
            let delta_type = base_object.get_type();
            let mut decompressed_reader = ReadBoxed::new_for_delta(&mut self.inner);
            let mut delta_reader = DeltaReader::new(&mut decompressed_reader, base_object).await;
            //let size = delta_reader.len();
            let re: Arc<dyn ObjectT> = match delta_type {
                ObjectType::Commit => Arc::new(Commit::new_delta(&mut delta_reader)),
                ObjectType::Tree => Arc::new(Tree::new_delta(&mut delta_reader)),
                ObjectType::Blob => Arc::new(Blob::new_delta(&mut delta_reader)),
                ObjectType::Tag => Arc::new(Tag::new_delta(&mut delta_reader)),
                _ => {
                    return Err(GitError::InvalidObjectType(
                        "from iterator:108,Unknown".to_string(),
                    ))
                }
            };
            iter_offset += decompressed_reader.decompressor.total_in() as usize;
            re
        };

        let result = obj.clone();
        let h = Arc::clone(&obj).get_hash();
        self.cache.put(self.offset, h, obj);
        self.offset += iter_offset;
        Ok(result)
    }

    pub fn read_tail_hash(&mut self) -> Hash {
        let id: [u8; 20] = {
            let mut id_buf = [0u8; 20];
            self.inner.read_exact(&mut id_buf).unwrap();
            id_buf
        };
        Hash::new_from_bytes(&id[..])
    }
}

async fn read_object(
    inner: &mut dyn BufRead,
    obj_type: ObjectType,
    size: usize,
    rsize: &mut usize,
) -> Result<Arc<dyn ObjectT>, GitError> {
    let mut decompressed_reader = ReadBoxed::new(inner, obj_type, size);
    let re: Result<Arc<dyn ObjectT>, GitError> = match obj_type {
        ObjectType::Commit => Ok(Arc::new(Commit::new_from_read(
            &mut decompressed_reader,
            size,
        ))),
        ObjectType::Tree => Ok(Arc::new(Tree::new_from_read(
            &mut decompressed_reader,
            size,
        ))),
        ObjectType::Blob => Ok(Arc::new(Blob::new_from_read(
            &mut decompressed_reader,
            size,
        ))),
        ObjectType::Tag => Ok(Arc::new(Tag::new_from_read(&mut decompressed_reader, size))),
        _ => Err(GitError::InvalidObjectType(
            "from iterator:109,Unknown".to_string(),
        )),
    };
    *rsize += decompressed_reader.decompressor.total_in() as usize;
    re
}
