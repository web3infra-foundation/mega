use std::borrow::Cow;
use std::collections::{hash_map, hash_set, HashMap, HashSet};
use std::iter;
use std::str::FromStr;
use libp2p::{PeerId};
use libp2p::kad::{K_VALUE, ProviderRecord, Record, RecordKey};
use libp2p::kad::store::{Error, RecordStore, Result};
use redis::{ErrorKind, from_redis_value, FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::{Serialize, Deserialize};
use kvcache::connector::redis::RedisClient;
use kvcache::KVCache;

pub struct DHTRedisStore {
    config: DHTRedisStoreConfig,
    provided: HashSet<ProviderRecord>,
    redis_cache: KVCache<RedisClient<String, DHTRedisRecord>>,
    records: HashMap<RecordKey, Record>,
}

#[derive(Debug, Clone)]
pub struct DHTRedisStoreConfig {
    pub max_records: usize,
    pub max_value_bytes: usize,
    pub max_providers_per_key: usize,
    pub max_provided_keys: usize,
}

impl Default for DHTRedisStoreConfig {
    fn default() -> Self {
        Self {
            max_records: 1024,
            max_value_bytes: 65 * 1024,
            max_provided_keys: 1024,
            max_providers_per_key: K_VALUE.get(),
        }
    }
}

impl Default for DHTRedisStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DHTRedisStore {
    pub fn new() -> Self {
        Self::with_config(Default::default())
    }

    pub fn with_config(config: DHTRedisStoreConfig) -> DHTRedisStore {
        DHTRedisStore {
            config,
            provided: HashSet::default(),
            redis_cache: KVCache::<RedisClient<String, DHTRedisRecord>>::new(),
            records: HashMap::default(),
        }
    }

    pub fn retain<F>(&mut self, _: F)
        where
            F: FnMut(&RecordKey, &mut Record) -> bool,
    {}
}

impl RecordStore for DHTRedisStore {
    type RecordsIter<'a> =
    iter::Map<hash_map::Values<'a, RecordKey, Record>, fn(&'a Record) -> Cow<'a, Record>>;

    type ProvidedIter<'a> = iter::Map<
        hash_set::Iter<'a, ProviderRecord>,
        fn(&'a ProviderRecord) -> Cow<'a, ProviderRecord>,
    >;

    fn get(&self, k: &RecordKey) -> Option<Cow<'_, Record>> {
        let key = from_record_key(k.clone());
        if let Some(dht_redis_record) = self.redis_cache.get(key) {
            let record = dht_redis_record.to_record(k.clone());
            return Some(Cow::Owned(record.clone()));
        }
        None
    }

    fn put(&mut self, r: Record) -> Result<()> {
        if r.value.len() >= self.config.max_value_bytes {
            return Err(Error::ValueTooLarge);
        }
        //check maxSize?
        let dht_redis_record = DHTRedisRecord::from_record(r);
        let _ = self.redis_cache.set(dht_redis_record.clone().key, dht_redis_record.clone());
        Ok(())
    }

    fn remove(&mut self, k: &RecordKey) {
        self.redis_cache.del(from_record_key(k.clone())).unwrap();
    }

    fn records(&self) -> Self::RecordsIter<'_> {
        self.records.values().map(Cow::Borrowed)
    }

    fn add_provider(&mut self, _: ProviderRecord) -> Result<()> {
        Ok(())
    }

    fn providers(&self, _: &RecordKey) -> Vec<ProviderRecord> {
        Vec::new()
    }

    fn provided(&self) -> Self::ProvidedIter<'_> {
        self.provided.iter().map(Cow::Borrowed)
    }

    fn remove_provider(&mut self, _: &RecordKey, _: &PeerId) {}
}

// assume a task is defined as "<id>-<desc>"
impl FromRedisValue for DHTRedisRecord {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let v: String = from_redis_value(v)?;
        match serde_json::from_str(v.as_str()) {
            Ok(dht_redis_record) => Ok(dht_redis_record),
            Err(_) => Err((ErrorKind::ResponseError, "json parse error").into()),
        }
    }
}

impl ToRedisArgs for DHTRedisRecord {
    fn write_redis_args<W>(&self, out: &mut W) where W: ?Sized + RedisWrite {
        out.write_arg_fmt(serde_json::to_string(self).expect("Can't serialize DHTRedisRecord as string"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct DHTRedisRecord {
    pub key: String,
    pub value: String,
    pub publisher: Option<String>,
    // pub expires: Option<i64>,
}

impl DHTRedisRecord {
    fn to_record(&self, record_key: RecordKey) -> Record {
        let mut record = Record::new(record_key, self.value.clone().into_bytes());
        if let Some(peer_id_str) = self.publisher.clone() {
            record.publisher.replace(PeerId::from_str(peer_id_str.as_str()).unwrap());
        }
        record
    }

    fn from_record(record: Record) -> DHTRedisRecord {
        let publisher = if record.publisher.is_some() {
            Some(record.publisher.unwrap().to_string())
        } else {
            None
        };

        DHTRedisRecord {
            key: from_record_key(record.key),
            value: String::from_utf8(record.value).unwrap(),
            publisher,
        }
    }
}


fn from_record_key(record_key: RecordKey) -> String {
    let prefix = "DHT_";
    let key = String::from_utf8(record_key.to_vec()).unwrap();
    format!("{}{}", prefix, key)
}
