use crate::utils;
use super::Connector;
use anyhow::Result;
use redis::{
    Connection, ConnectionInfo, FromRedisValue,ToRedisArgs, IntoConnectionInfo,
};
use std::{cell::RefCell, marker::PhantomData};

pub struct RedisClient<K, V> {
    conn: RefCell<Connection>,
    k: PhantomData<K>,
    v: PhantomData<V>,
}

impl<K, V> Connector for RedisClient<K, V>
where
    K: ToRedisArgs,
    V: ToRedisArgs + FromRedisValue,
{
    type K = K;
    type V = V;
    fn get(&self, key: Self::K) -> Option<Self::V> {
        match redis::cmd("GET")
            .arg(key)
            .query(&mut self.conn.borrow_mut())
        {
            Ok(a) => Some(a),
            Err(_) => None,
        }
    }

    fn set(&self, key: Self::K, v: Self::V) -> Result<()> {
        match redis::cmd("SET")
            .arg(key)
            .arg(v)
            .query::<bool>(&mut self.conn.borrow_mut())
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
    fn new() -> RedisClient<K, V> {
        let mut addr: String= String::new();
        utils::get_env_number("REDIS_CONFIG", &mut addr);
        let config :ConnectionInfo  = addr.into_connection_info().unwrap();
        let mut c = Self::new_client(config).unwrap();

        let _ = redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory")
        .arg("2G")
        .query::<bool>(&mut c).unwrap();

        let _ = redis::cmd("CONFIG")
        .arg("SET")
        .arg("maxmemory-policy")
        .arg("allkeys-lru")
        .query::<bool>(&mut c).unwrap();

        RedisClient {
            conn: RefCell::new(c),
            k: PhantomData,
            v: PhantomData,
        }
    }
}
impl<K, V> RedisClient<K, V>
where
    K: ToRedisArgs,
    V: ToRedisArgs + FromRedisValue,
{
    fn new_client(info: ConnectionInfo) -> Result<Connection> {
        
        let client = redis::Client::open(info)?;
        let con = client.get_connection()?;
        // let ss:RefCell<C> = RefCell::new(con);
        Ok(con)
    }
}

#[cfg(test)]
mod tests {
    use crate::connector::Connector;
    use crate::KVCache;
    use anyhow::Result;
    use redis::{ErrorKind, FromRedisValue, RedisError, ToRedisArgs, cmd};
    use redis_test::{MockCmd, MockRedisConnection};
    use serde::{Deserialize, Serialize};
    use std::{cell::RefCell, marker::PhantomData, vec};

    #[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
    struct TestMessage {
        id: u32,
        message: Vec<u8>,
    }
    impl ToRedisArgs for TestMessage {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            out.write_arg(&serde_json::to_vec(self).unwrap())
        }
    }
    impl FromRedisValue for TestMessage {
        fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
            println!("{:?}", v);
            match v {
                redis::Value::Nil => Err(RedisError::from((ErrorKind::TypeError, "nil value "))),
                redis::Value::Int(_) => {
                    Err(RedisError::from((ErrorKind::TypeError, "cat by int  ")))
                }
                redis::Value::Data(a) => {
                    if let Ok(message) = serde_json::from_slice::<Self>(a) {
                        Ok(message)
                    } else {
                        Err(RedisError::from((
                            ErrorKind::TypeError,
                            "cat conver by data cause json error  ",
                        )))
                    }
                }
                redis::Value::Bulk(_) => {
                    Err(RedisError::from((ErrorKind::TypeError, "cat by Bulk ")))
                }
                redis::Value::Status(_) => {
                    Err(RedisError::from((ErrorKind::TypeError, "nil value ")))
                }
                redis::Value::Okay => Err(RedisError::from((ErrorKind::TypeError, "nil value "))),
            }
        }
    }

    pub struct RedisMockClient<K, V> {
        conn: RefCell<MockRedisConnection>,
        k: PhantomData<K>,
        v: PhantomData<V>,
    }

    impl<K, V> Connector for RedisMockClient<K, V>
    where
        K: ToRedisArgs,
        V: ToRedisArgs + FromRedisValue,
    {
        type K = K;
        type V = V;
        fn get(&self, key: Self::K) -> Option<Self::V> {
            match redis::cmd("GET")
                .arg(key)
                .query(&mut self.conn.borrow_mut())
            {
                Ok(a) => Some(a),
                Err(err) => {
                    print!("redis set err:{}",err);
                    None
                },
            }
        }

        fn set(&self, key: Self::K, v: Self::V) -> Result<()> {
            match redis::cmd("SET")
                .arg(key)
                .arg(v)
                .query::<()>(&mut self.conn.borrow_mut())
            {
                Ok(_) => Ok(()),
                Err(err) => Err(err.into()),
            }
        }

        fn new() -> Self {
            let c = Self::new_client().unwrap();
            RedisMockClient {
                conn: RefCell::new(c),
                k: PhantomData,
                v: PhantomData,
            }
        }
    }

    impl<K, V> RedisMockClient<K, V>
    where
        K: ToRedisArgs,
        V: ToRedisArgs + FromRedisValue,
    {

        fn new_client() -> Result<MockRedisConnection> {
            let a  = TestMessage {
                id: 12,
                message: vec![1, 2, 3, 4, 5],
            };
            let b = TestMessage {
                id: 12,
                message: vec![4, 5, 6, 7, 8],
            };
            let connect = MockRedisConnection::new(vec![
                MockCmd::new(cmd("SET").arg(3).arg(a.clone()), Ok("")),
                MockCmd::new(cmd("SET").arg(4).arg(b.clone()), Ok("")),
                MockCmd::new(cmd("GET").arg(3),Ok(serde_json::to_vec(&a).unwrap())),
                MockCmd::new(cmd("GET").arg(4), Ok(serde_json::to_vec(&b).unwrap())),
            ]);
            
            Ok(connect)
        }
    }

    #[test]
    fn test_mock_redis() {
        let cache = KVCache::<RedisMockClient<_, _>>::new();
        let a = TestMessage {
            id: 12,
            message: vec![1, 2, 3, 4, 5],
        };
        let b = TestMessage {
            id: 12,
            message: vec![4, 5, 6, 7, 8],
        };
        cache.set(3_i32, a.clone()).unwrap();
        cache.set(4, b.clone()).unwrap();
        assert_eq!(cache.get(3), Some(a));
        assert_eq!(cache.get(4), Some(b));
    }
}
