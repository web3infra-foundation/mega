use super::{cache::ObjectCache, counter::GitTypeCounter, delta::undelta, EntryHeader, Pack};
use crate::{
    errors::GitError,
    internal::{
        pack::{counter::DecodeCounter, Hash, cqueue::CircularQueue},
        zlib::stream::inflate::ReadPlain,
    },
    utils,
};
use async_recursion::async_recursion;
use database::{driver::ObjectStorage, utils::id_generator::generate_id};
use entity::{git, obj_data};
use num_cpus;

use sea_orm::Set;
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::RwLock;

///
/// One Pre loading Git object in memory
///
#[derive(Clone)]
struct Entry {
    header: EntryHeader,
    offset: usize,
    data: Vec<u8>,
    hash: Option<Hash>,
}
impl Entry {
    fn convert_to_mr_model(self, mr_id: i64) -> git::ActiveModel {
        git::ActiveModel {
            id: Set(generate_id()),
            mr_id: Set(mr_id),
            git_id: Set(self.hash.unwrap().to_plain_str()),
            object_type: Set(String::from_utf8_lossy(self.header.to_bytes()).to_string()),
            created_at: Set(chrono::Utc::now().naive_utc()),
        }
    }
    fn convert_to_data_model(self) -> obj_data::ActiveModel {
        obj_data::ActiveModel {
            id: Set(generate_id()),
            git_id: Set(self.hash.unwrap().to_plain_str()),
            object_type: Set(String::from_utf8_lossy(self.header.to_bytes()).to_string()),
            data: Set(self.data),
        }
    }
}

/// All Git Objects pre loading in memeory of one pack file.
pub struct PackPreload {
    map: HashMap<usize, usize>, //Offset -> iterator in entity
    entries: Vec<Entry>,        // store git entries by vec.
    counter: GitTypeCounter,
}

#[allow(unused)]
impl PackPreload {
    pub fn new<R>(mut r: R) -> PackPreload
    where
        R: std::io::BufRead,
    {
        let start = Instant::now();
        let mut offset: usize = 12;
        // Object Types Counter
        let mut counter = GitTypeCounter::default();
        let pack = Pack::check_header(&mut r).unwrap();
        // Offset - index in vec Map for preload struct
        let mut map = HashMap::new();
        let obj_number = pack.number_of_objects();
        let mut entries = Vec::with_capacity(obj_number);
        tracing::info!("Start Preload git objects:{} ", obj_number);
        for i in 0..obj_number {
            if i % 10000 == 0 {
                tracing::info!(" Preloading  git objects:{} ", i);
            }
            // [`iter_offset`] records the number of bytes occupied by a single object.
            let mut iter_offset: usize = 0;
            // Read the Object Type and Total Size of one Object
            let (type_num, size) = utils::read_type_and_size(&mut r).unwrap();
            //Get the Object according to the Types Enum
            iter_offset += utils::get_7bit_count(size << 3);
            // Count Type
            counter.count(type_num);
            let header: EntryHeader = match type_num {
                1 => EntryHeader::Commit,
                2 => EntryHeader::Tree,
                3 => EntryHeader::Blob,
                4 => EntryHeader::Tag,

                6 => {
                    // Offset Delta Object
                    let delta_offset =
                        utils::read_offset_encoding(&mut r, &mut iter_offset).unwrap() as usize;

                    // Count the base object offset and get the base object from the cache in EntriesIter
                    let base_offset = offset
                        .checked_sub(delta_offset)
                        .ok_or_else(|| {
                            GitError::InvalidObjectInfo("Invalid OffsetDelta offset".to_string())
                        })
                        .unwrap();
                    EntryHeader::OfsDelta {
                        base_distance: base_offset,
                    }
                }
                7 => {
                    // Ref Delta Object
                    let hash = utils::read_hash(&mut r).unwrap();
                    iter_offset += 20;
                    EntryHeader::RefDelta { base_id: hash }
                }
                _ => todo!(), //error
            };
            let mut reader = ReadPlain::new(&mut r);
            // init vec by given size.
            let mut content = Vec::with_capacity(size);
            reader.read_to_end(&mut content).unwrap();
            iter_offset += reader.decompressor.total_in() as usize;

            //println!("offset :{},type :{}",offset,type_num);
            entries.push(Entry {
                header,
                offset,
                data: content,
                hash: None,
            });
            map.insert(offset, i);
            offset += iter_offset;
        }
        let end = start.elapsed().as_millis();
        tracing::info!("Preload time cost:{} ms", end);
        PackPreload {
            map,
            entries,
            counter,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Decide the preloaded objects, and store it .
/// `decode_load` function used for decoding and loading data.
///
/// The `decode_load` function takes a `PackPreload` parameter and an `ObjectStorage` parameter.
/// It asynchronously decodes and loads data from the provided `PackPreload` using the given `ObjectStorage`.
///
/// # Arguments
///
/// - `p`: A `PackPreload` struct representing the data to be decoded and loaded.
/// - `storage`: An `Arc<dyn ObjectStorage>` trait object providing storage capabilities.
///
/// # Returns
///
/// The function returns a `Result<i64, GitError>`, where the `i64` represents the `mr_id`
/// and `GitError` represents any potential error that might occur during the process.
///
pub async fn decode_load(p: PackPreload, storage: Arc<dyn ObjectStorage>) -> Result<i64, GitError> {
    let decode_counter: Arc<Mutex<DecodeCounter>> = Arc::new(Mutex::new(DecodeCounter::default()));
    let all_len = p.len();
    tracing::info!("Decode the preload git object\n{}", p.counter);
    let (cpu_number, chunk) = thread_chunk(all_len);
    tracing::info!("Deal with the object using {} threads. ", cpu_number);
    let share: Arc<RwLock<PackPreload>> = Arc::new(RwLock::new(p));

    let mr_id = generate_id();

    let producer_handles: Vec<_> = (0..cpu_number)
        .map(|i| {
            let shard_clone = Arc::clone(&share);
            let st_clone = storage.clone();
            let counter_clone = decode_counter.clone();
            let begin = i * chunk;
            let end = if i == cpu_number - 1 {
                all_len
            } else {
                (i + 1) * chunk
            };
            tokio::spawn(async move {
                produce_object(shard_clone, st_clone, begin, end, counter_clone, mr_id).await;
            })
        })
        .collect();

    for handle in producer_handles {
        let _ = handle.await;
    }

    let re = decode_counter.lock().unwrap();
    tracing::info!("Summary : {}", re);

    Ok(mr_id)
}

use super::counter::CounterType::*;
/// Asynchronous function to produce Git objects.
///
/// The `produce_object` function asynchronously produces Git objects based on the provided parameters.
///
/// # Arguments
///
/// - `data`: A shared `Arc<RwLock<PackPreload>>` containing the preload data.
/// - `storage`: A shared `Arc<dyn ObjectStorage>` trait object providing storage capabilities.
/// - `range_begin`: The starting index of the range of entries to process.
/// - `range_end`: The ending index of the range of entries to process.
/// - `counter`: A shared `Arc<Mutex<DecodeCounter>>` for counting decode operations.
/// - `mr_id`: An identifier for the produced Git objects.
///
async fn produce_object(
    data: Arc<RwLock<PackPreload>>,
    storage: Arc<dyn ObjectStorage>,
    range_begin: usize,
    range_end: usize,
    counter: Arc<Mutex<DecodeCounter>>,
    mr_id: i64,
) {
    let mut git_save_model = Vec::<git::ActiveModel>::with_capacity(1001);
    let mut data_save_model = Vec::<obj_data::ActiveModel>::with_capacity(1001);

    let mut object_cache_size = 1000;
    utils::get_env_number("GIT_INTERNAL_DECODE_CACHE_SIZE", &mut object_cache_size);

    let mut cache: ObjectCache<Entry> = ObjectCache::new(Some(object_cache_size));
    let start = Instant::now();
    let mut batch_size = 10000;
    utils::get_env_number("GIT_INTERNAL_DECODE_STORAGE_BATCH_SIZE", &mut batch_size);

    let mut save_task_wait_number=10; // the most await save thread amount
    utils::get_env_number("GIT_INTERNAL_DECODE_STORAGE_TQUEUE_SIZE", &mut save_task_wait_number);

    let mut  save_queue: CircularQueue<_> = CircularQueue::new(save_task_wait_number);
    for i in range_begin..range_end {
        let read_auth = data.read().await;
        let e = &read_auth.entries[i];

        let result_entity;
        match e.header {
            EntryHeader::RefDelta { base_id } => {
                let base_type;
                let base_data = if let Some(b_obj) = cache.get_by_hash(base_id) {
                    {
                        counter.lock().unwrap().count(CacheHit);
                    }
                    base_type = b_obj.header;
                    b_obj.data
                } else {
                    let db_obj = storage
                        .get_obj_data_by_id(&base_id.to_plain_str())
                        .await
                        .unwrap()
                        .unwrap();
                    base_type = EntryHeader::from_string(&db_obj.object_type);
                    {
                        counter.lock().unwrap().count(DB);
                    }
                    db_obj.data
                };

                let re = undelta(&mut Cursor::new(e.data.clone()), &base_data);
                let undelta_obj = Entry {
                    header: base_type,
                    offset: e.offset,
                    data: re,
                    hash: None,
                };
                result_entity = compute_hash(undelta_obj);
                {
                    counter.lock().unwrap().count(Delta);
                }
            }
            EntryHeader::OfsDelta { base_distance: _ } => {
                let re_obj = delta_offset_obj(data.clone(), e, &mut cache, counter.clone()).await;
                result_entity = compute_hash(re_obj);
                {
                    counter.lock().unwrap().count(Delta);
                }
            }
            _ => {
                {
                    counter.lock().unwrap().count(Base);
                }
                result_entity = compute_hash(e.clone());
            }
        }
        cache.put(e.offset, result_entity.hash.unwrap(), result_entity.clone());
        git_save_model.push(result_entity.clone().convert_to_mr_model(mr_id));
        data_save_model.push(result_entity.convert_to_data_model());

        //save to storage 
       
        if git_save_model.len()>=batch_size{
            let stc = storage.clone();
            let h = tokio::spawn(async move {
                stc.save_git_objects(git_save_model).await.unwrap();
                stc.save_obj_data(data_save_model).await.unwrap();
            });
            println!("put new batch ");
            // if the save queue if full , wait the fist queue finish
            if save_queue.is_full(){
                let first_h: tokio::task::JoinHandle<()> = save_queue.dequeue().unwrap();
                println!("to await from full queue ");
                first_h.await.unwrap();
                save_queue.enqueue(h).unwrap();
            }else {
                save_queue.enqueue(h).unwrap();
            }
            git_save_model  = Vec::with_capacity(batch_size);
            data_save_model = Vec::with_capacity(batch_size); 
        }
    }
    if !git_save_model.is_empty() {
        storage.save_git_objects(git_save_model).await.unwrap();
    }
    if !data_save_model.is_empty() {
        storage.save_obj_data(data_save_model).await.unwrap();
    }
    // await the remaining threads
    println!("await last thread");
    while let Some(h)= save_queue.dequeue() {
        h.await.unwrap();
    }
    let end = start.elapsed().as_millis();
    tracing::info!("Git Object Produce thread one  time cost:{} ms", end);
}

/// Asynchronous function to perform delta offset operation.
///
/// The `delta_offset_obj` function asynchronously performs the delta offset operation on the given data.
///
/// # Arguments
///
/// - `data`: A shared `Arc<RwLock<PackPreload>>` containing the preload data.
/// - `delta_obj`: A reference to the `Entry` representing the delta object to process.
/// - `cache`: A mutable reference to the `ObjectCache<Entry>` used for caching objects.
/// - `counter`: A shared `Arc<Mutex<DecodeCounter>>` for counting decode operations.
///
/// # Returns
///
/// The function returns an `Entry` representing the result of the delta offset operation.
///
#[async_recursion] //TODO del recursion
async fn delta_offset_obj(
    data: Arc<RwLock<PackPreload>>,
    delta_obj: &Entry,
    cache: &mut ObjectCache<Entry>,
    counter: Arc<Mutex<DecodeCounter>>,
) -> Entry {
    let share = data.read().await;
    if let EntryHeader::OfsDelta { base_distance } = delta_obj.header {
        let basic_type;
        let base_obj;
        let buff_obj;
        if let Some(b_obj) = cache.get(base_distance) {
            {
                counter.lock().unwrap().count(CacheHit);
            }
            buff_obj = b_obj;
            base_obj = &buff_obj;
        } else {
            let pos = share.map.get(&base_distance).unwrap();
            base_obj = &share.entries[*pos];
        }

        let re;
        // check its weather need to deeper recursion
        if !base_obj.header.is_base() {
            {
                counter.lock().unwrap().count(Depth);
            }
            let d_obj = delta_offset_obj(data.clone(), base_obj, cache, counter).await;
            re = undelta(&mut Cursor::new(&delta_obj.data), &d_obj.data);
            basic_type = d_obj.header;
        } else {
            basic_type = base_obj.header.clone();
            re = undelta(&mut Cursor::new(&delta_obj.data), &base_obj.data);
        }

        Entry {
            header: basic_type,
            offset: delta_obj.offset,
            data: re,
            hash: None,
        }
    } else {
        panic!("cat't call by base obj ");
    }
}

fn compute_hash(mut e: Entry) -> Entry {
    match e.header {
        EntryHeader::RefDelta { base_id: _ } => panic!("this methon can't call by delta"),
        EntryHeader::OfsDelta { base_distance: _ } => panic!("this methon can't call by delta"),
        _ => (),
    }

    let mut h = Sha1::new();
    h.update(e.header.to_bytes());
    h.update(b" ");
    h.update(e.data.len().to_string());
    h.update(b"\0");
    h.update(&e.data);
    let re: [u8; 20] = h.finalize().into();
    e.hash = Some(Hash(re));
    e
}

fn thread_chunk(len: usize) -> (usize, usize) {
    let cpu_number: usize = num_cpus::get();
    if len < cpu_number {
        (cpu_number, 0)
    } else {
        (cpu_number, len / cpu_number)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader, path::Path};

    use crate::internal::pack::preload::PackPreload;
    use tokio::test;

    #[test]
    async fn preload_read_decode() {
        let file = File::open(Path::new(
            "../tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack",
        ))
        .unwrap();

        let p = PackPreload::new(BufReader::new(file));
        println!("{}", p.len());
        for it in p.entries {
            println!("{:?},offset:{}", it.header, it.offset);
        }
    }

    #[test]
    #[ignore]
    async fn test_demo_channel() {
        std::env::set_var(
            "MEGA_DATABASE_URL",
            "mysql://root:123456@localhost:3306/mega",
        );
        let file = File::open(Path::new(
            "/home/99211/linux/.git/objects/pack/pack-a3f96bcba83583d37b77a528b82bd1d97ffac70c.pack",
        ))
        .unwrap();
        let p = PackPreload::new(BufReader::new(file));
        let mut total = 0;
        for it in p.entries {
            total += it.data.len();
        }
        println!("{}", total);
    }
}
