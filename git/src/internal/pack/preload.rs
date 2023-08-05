
use super::{cache::ObjectCache, delta::undelta, Pack, EntryHeader, counter::GitTypeCounter};
use crate::{
    errors::GitError,
    internal::{pack::{Hash, counter::DecodeCounter}, zlib::stream::inflate::ReadPlain},
    utils,
};
use async_recursion::async_recursion;
use database::{driver::ObjectStorage, utils::id_generator::generate_id};
use entity::git;
use num_cpus;

use sea_orm::Set;
use sha1::{Digest, Sha1};
use tokio::sync::RwLock;
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    sync::{mpsc, Arc,Mutex},
};

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
            data: Set(self.data),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }
}


/// All Git Objects pre loading in memeory of one pack file.
pub struct PackPreload {
    map: HashMap<usize, usize>, //Offset -> iterator in entity
    entries: Vec<Entry>,// store git entries by vec.
    counter: GitTypeCounter,
}

#[allow(unused)]
impl PackPreload {
    pub fn new<R>(mut r: R) -> PackPreload
    where
        R: std::io::BufRead,
    {
        let mut offset: usize = 12;
        let mut counter = GitTypeCounter::default();
        let pack = Pack::check_header(&mut r).unwrap();
        let mut map = HashMap::new();
        let obj_number = pack.number_of_objects();
        let mut entries = Vec::with_capacity(obj_number);
        for i in 0..obj_number {
            let mut iter_offset: usize = 0;
            // Read the Object Type and Total Size of one Object
            let (type_num, size) = utils::read_type_and_size(&mut r).unwrap();
            //Get the Object according to the Types Enum
            iter_offset += utils::get_7bit_count(size << 3);
            counter.count(type_num);
            let header: EntryHeader = match type_num {
                1 => EntryHeader::Commit,
                2 => EntryHeader::Tree,
                3 => EntryHeader::Blob,
                4 => EntryHeader::Tag,

                6 => {
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
        PackPreload { map, entries ,counter }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool{
        self.entries.is_empty()
    }
}


#[allow(unused)]
pub async fn decode_load(p: PackPreload, storage: Arc<dyn ObjectStorage>) {
    let decode_counter: Arc<Mutex<DecodeCounter>> = Arc::new(Mutex::new(DecodeCounter::default())) ;

    let all_len = p.len();
    tracing::info!("Decode the preload git object\n{}",p.counter);
          println!("Decode the preload git object\n{}",p.counter);
    let (cpu_number, chunk) = thread_chunk(all_len);
    
    tracing::info!("Deal with the object using {} threads. ",cpu_number);
          println!("Deal with the object using {} threads. ",cpu_number);
    let share: Arc<RwLock<PackPreload>> = Arc::new(RwLock::new(p));
    // 创建一个多生产者，单消费者的通道
    let (tx, rx) = mpsc::channel();

    let producer_handles: Vec<_> = (0..cpu_number)
        .map(|i| {
            let tx_clone = tx.clone();
            let shard_clone = Arc::clone(&share) ;
            let st_clone = storage.clone();
            let counter_clone = decode_counter.clone();
            //produce_data(i, tx_clone)
            let begin = i * chunk;
            let end = if i == cpu_number - 1 {
                all_len
            } else {
                (i + 1) * chunk
            };
            tokio::spawn(async move{
                produce_object(shard_clone, tx_clone, st_clone, begin, end,counter_clone).await;
            } )
        })
        .collect();

    // 启动消费者任务
    let consume_handle = tokio::spawn(async move  {
        consume_object(rx, storage.clone()).await;
    });

    // 等待所有生产者任务结束
    for handle in producer_handles {
        handle.await;
    }

    // 关闭通道以结束消费者任务
    drop(tx);

    // 等待消费者任务结束
    consume_handle.await;
    let re = decode_counter.lock().unwrap();
    tracing::info!("Summary : {}",re);
          println!("Summary : {}",re);
}

use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use super::counter::CounterType::*;
async fn produce_object(
    data: Arc<RwLock<PackPreload>>,
    send: Sender<Entry>,
    storage: Arc<dyn ObjectStorage>,
    range_begin: usize,
    range_end: usize,
    counter: Arc<Mutex<DecodeCounter>>,
) {
    let mut cache: ObjectCache<Entry> = ObjectCache::new(Some(100));

    
    for i in range_begin..range_end {
        let read_auth = data.read().await;
        let e = &read_auth.entries[i];
        //todo hash .
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
                    let db_obj =storage.get_git_object_by_hash(&base_id.to_plain_str()).await 
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
            _ =>{
                {
                    counter.lock().unwrap().count(Base);
                }
                result_entity = compute_hash(e.clone());
            } 
        }
        cache.put(e.offset, result_entity.hash.unwrap(), result_entity.clone());
        send.send(result_entity).unwrap();

        // TYPE HASH DATA  // DELTA
    }
}

async fn consume_object(rx: Receiver<Entry>, storage: Arc<dyn ObjectStorage>) {
    let mut save_model = Vec::<git::ActiveModel>::with_capacity(101);
    
    let mr_id = generate_id();
   
    for data in rx.into_iter() {
        save_model.push(data.convert_to_mr_model(mr_id));
        if save_model.len() >= 100 {
            storage.save_git_objects(save_model).await.unwrap();
            save_model = Vec::with_capacity(101);
        }
    }
    if !save_model.is_empty() {
        storage.save_git_objects(save_model).await.unwrap();
    }
}
#[async_recursion]//TODO del recursion
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
            let d_obj =  delta_offset_obj(data.clone(), base_obj, cache,counter).await;
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

    use super::decode_load;
    use database::DataSource;
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
        std::env::set_var("MEGA_DATABASE_URL", "mysql://root:123456@localhost:3306/mega");
        let file = File::open(Path::new(
            "../tests/data/packs/pack-d50df695086eea6253a237cb5ac44af1629e7ced.pack",
        ))
        .unwrap();
        let storage = database::init(&DataSource::Mysql).await;
        let p = PackPreload::new(BufReader::new(file));
        decode_load(p, storage).await;
    }
}
