use database::driver::ObjectStorage;

use crate::{
    internal::{
        object::{blob::Blob, commit::Commit, from_model, tag::Tag, tree::Tree, GitObjects},
        pack::delta::DeltaReader,
        zlib::stream::inflate::ReadBoxed,
        GitError, ObjectType,
    },
    utils,
};

use crate::internal::object::{cache::ObjectCache, ObjectT};
use std::sync::Arc;
type IteratorResult = Result<Arc<dyn ObjectT>, GitError>;
type GitIteratorResult = Result<GitObjects, GitError>;

///
pub struct EntriesIter<BR> {
    inner: BR,
    offset: usize,
    objects_left: u32,
    cache: ObjectCache,
    storage: Option<Arc<dyn ObjectStorage>>,
}

impl<BR: std::io::BufRead> EntriesIter<BR> {
    //After Pack::check_header
    pub fn new(r: BR, obj_num: u32) -> Self {
        Self {
            inner: r,
            offset: 12,
            objects_left: obj_num,
            cache: ObjectCache::new(),
            storage: None,
        }
    }
    pub fn set_storage(&mut self, s: Option<Arc<dyn ObjectStorage>>) {
        self.storage = s;
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
            let mut decompressed_reader = ReadBoxed::new(&mut self.inner, obj_type, size);
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
            iter_offset += decompressed_reader.decompressor.total_in() as usize;
            re
        } else {
            let base_object: Arc<dyn ObjectT>;

            if type_num == 6 {
                // Offset Delta Object
                let offset = self.offset;
                let delta_offset = utils::read_offset_encoding(&mut self.inner, &mut iter_offset)
                    .unwrap() as usize;
                //iter_offset += utils::get_7bit_count(delta_offset);
                // Count the base object offset and get the base object from the cache in EntriesIter
                let base_offset = offset
                    .checked_sub(delta_offset)
                    .ok_or_else(|| {
                        GitError::InvalidObjectInfo("Invalid OffsetDelta offset".to_string())
                    })
                    .unwrap();

                if let Some(bo) = self.cache.get(base_offset) {
                    base_object = bo;
                } else {
                    let base_hash = self.cache.get_hash(base_offset).unwrap();
                    if let Some(storage) = &self.storage {
                        let _model = storage
                            .get_git_object_by_hash(&base_hash.to_plain_str())
                            .await
                            .unwrap()
                            .ok_or_else(|| {
                                tracing::error!("invalid base offset: {}, invalid hash: {}", base_offset, base_hash.to_plain_str());
                                GitError::DeltaObjectError(
                                    "cant' find base obj from offset".to_string(),
                                )
                            })?; //TODO: Handler mega error to Git Error?
                        base_object = from_model(_model);
                    } else {
                        return Err(GitError::DeltaObjectError(
                            "we don't have a storage ".to_string(),
                        ));
                    }
                }
            } else if type_num == 7 {
                // Ref Delta Object
                let hash = utils::read_hash(&mut self.inner).unwrap();
                iter_offset += 20;

                if let Some(bo) = self.cache.get_by_hash(hash) {
                    base_object = bo;
                } else if let Some(storage) = &self.storage {
                    let _model = storage
                        .get_git_object_by_hash(&hash.to_plain_str())
                        .await
                        .unwrap()
                        .ok_or_else(|| {
                            println!("wrong base hash value :{}", hash);
                            GitError::DeltaObjectError(
                                "cant' find base obj from hash value ".to_string(),
                            )
                        })?; //TODO: Handler mega error to Git Error?
                    base_object = from_model(_model);
                } else {
                    return Err(GitError::DeltaObjectError(
                        "we don't have a storage ".to_string(),
                    ));
                };
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
            Ok(re)
        }?;

        let result = obj.clone();
        let h = Arc::clone(&obj).get_hash();
        self.cache.put(self.offset, h, obj);
        self.offset += iter_offset;
        Ok(result)
    }

    pub async fn next_git_obj(&mut self) -> GitIteratorResult {
        self.objects_left -= 1;
        let mut iter_offset: usize = 0;
        // Read the Object Type and Total Size of one Object
        let (type_num, size) = utils::read_type_and_size(&mut self.inner).unwrap();
        //Get the Object according to the Types Enum
        let obj_type = ObjectType::number2type(type_num).unwrap();
        iter_offset += utils::get_7bit_count(size << 3);

        let obj: GitObjects = if (1..=4).contains(&type_num) {
            let mut decompressed_reader = ReadBoxed::new(&mut self.inner, obj_type, size);
            let re: Result<GitObjects, GitError> = match obj_type {
                ObjectType::Commit => Ok(GitObjects::COMMIT(Commit::new_from_read(
                    &mut decompressed_reader,
                    size,
                ))),
                ObjectType::Tree => Ok(GitObjects::TREE(Tree::new_from_read(
                    &mut decompressed_reader,
                    size,
                ))),
                ObjectType::Blob => Ok(GitObjects::BLOB(Blob::new_from_read(
                    &mut decompressed_reader,
                    size,
                ))),
                ObjectType::Tag => Ok(GitObjects::TAG(Tag::new_from_read(
                    &mut decompressed_reader,
                    size,
                ))),
                _ => Err(GitError::InvalidObjectType(
                    "from iterator:109,Unknown".to_string(),
                )),
            };
            iter_offset += decompressed_reader.decompressor.total_in() as usize;
            re
        } else {
            let base_object: Arc<dyn ObjectT>;

            if type_num == 6 {
                // Offset Delta Object
                let offset = self.offset;
                let delta_offset = utils::read_offset_encoding(&mut self.inner, &mut iter_offset)
                    .unwrap() as usize;
                //iter_offset += utils::get_7bit_count(delta_offset);
                // Count the base object offset and get the base object from the cache in EntriesIter
                let base_offset = offset
                    .checked_sub(delta_offset)
                    .ok_or_else(|| {
                        GitError::InvalidObjectInfo("Invalid OffsetDelta offset".to_string())
                    })
                    .unwrap();

                if let Some(bo) = self.cache.get(base_offset) {
                    base_object = bo;
                } else {
                    let base_hash = self.cache.get_hash(base_offset).unwrap();
                    if let Some(storage) = &self.storage {
                        let _model = storage
                            .get_git_object_by_hash(&base_hash.to_plain_str())
                            .await
                            .unwrap()
                            .ok_or_else(|| {
                                println!("wrong base offset :{}", base_offset);
                                GitError::DeltaObjectError(
                                    "cant' find base obj from offset".to_string(),
                                )
                            })?; //TODO: Handler mega error to Git Error?
                        base_object = from_model(_model);
                    } else {
                        return Err(GitError::DeltaObjectError(
                            "we don't have a storage ".to_string(),
                        ));
                    }
                }
            } else if type_num == 7 {
                // Ref Delta Object
                let hash = utils::read_hash(&mut self.inner).unwrap();
                iter_offset += 20;

                if let Some(bo) = self.cache.get_by_hash(hash) {
                    base_object = bo;
                } else if let Some(storage) = &self.storage {
                    let _model = storage
                        .get_git_object_by_hash(&hash.to_plain_str())
                        .await
                        .unwrap()
                        .ok_or_else(|| {
                            println!("wrong base hash value :{}", hash);
                            GitError::DeltaObjectError(
                                "cant' find base obj from hash value ".to_string(),
                            )
                        })?; //TODO: Handler mega error to Git Error?
                    base_object = from_model(_model);
                } else {
                    return Err(GitError::DeltaObjectError(
                        "we don't have a storage ".to_string(),
                    ));
                }
            } else {
                return Err(ObjectType::number2type(type_num).err().unwrap());
            }
            let delta_type = base_object.get_type();
            let mut decompressed_reader = ReadBoxed::new_for_delta(&mut self.inner);
            let mut delta_reader = DeltaReader::new(&mut decompressed_reader, base_object).await;
            //let size = delta_reader.len();
            let re: GitObjects = match delta_type {
                ObjectType::Commit => GitObjects::COMMIT(Commit::new_delta(&mut delta_reader)),
                ObjectType::Tree => GitObjects::TREE(Tree::new_delta(&mut delta_reader)),
                ObjectType::Blob => GitObjects::BLOB(Blob::new_delta(&mut delta_reader)),
                ObjectType::Tag => GitObjects::TAG(Tag::new_delta(&mut delta_reader)),
                _ => {
                    return Err(GitError::InvalidObjectType(
                        "from iterator:108,Unknown".to_string(),
                    ))
                }
            };
            iter_offset += decompressed_reader.decompressor.total_in() as usize;
            Ok(re)
        }?;
        let h: crate::hash::Hash;

        match obj.clone() {
            GitObjects::COMMIT(a) => {
                h = a.get_hash();
                self.cache.put(self.offset, h, Arc::new(a));
            }
            GitObjects::TREE(a) => {
                h = a.get_hash();
                self.cache.put(self.offset, h, Arc::new(a));
            }
            GitObjects::BLOB(a) => {
                h = a.get_hash();
                self.cache.put(self.offset, h, Arc::new(a));
            }
            GitObjects::TAG(a) => {
                h = a.get_hash();
                self.cache.put(self.offset, h, Arc::new(a));
            }
        };

        self.offset += iter_offset;
        Ok(obj)
    }
}
