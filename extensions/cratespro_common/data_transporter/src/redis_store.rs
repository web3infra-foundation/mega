use std::env;

use redis::{Commands, Connection};

pub struct RedisHandler {
    pub connection: Connection,
}

pub async fn get_redis_connection() -> Result<Connection, Box<dyn std::error::Error>> {
    let host = env::var("REDIS_HOST").unwrap_or_else(|_| "172.17.0.1".to_string());
    let password = env::var("REDIS_PASSWORD").unwrap_or_else(|_| "".to_string());
    println!("host:{host}");
    // 构建连接字符串
    let conn_string = format!("redis://:{password}@{host}:6379/");
    println!("conn_string:{conn_string}");
    println!("尝试连接 Redis: {conn_string}");

    // 创建客户端
    let client = redis::Client::open(&*conn_string).unwrap();
    println!("finish get client");
    // 获取连接
    let conn = client.get_connection().unwrap();
    Ok(conn)
}

impl RedisHandler {
    pub fn get_connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
    pub async fn query_from_redis(
        &mut self,
        qid: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        match self.get_connection_mut().get::<_, String>(&qid) {
            Ok(value) => Ok(value),
            Err(e) => {
                if e.kind() == redis::ErrorKind::TypeError {
                    Ok("".to_string())
                } else {
                    Err(Box::new(e))
                }
            }
        }
    }

    pub async fn insert_crates_info_into_redis(
        &mut self,
        namespace: String,
        name: String,
        version: String,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("start insert crates_info");
        let key = format!("crates_info:{namespace}:{name}:{version}");

        // 使用管道设置值和过期时间
        let _: () = redis::pipe()
            .cmd("SET")
            .arg(&key)
            .arg(&value)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(7 * 24 * 60 * 60) // 一周的秒数
            .query(&mut self.connection)?;
        println!("finish insert crates_info");
        Ok(())
    }
    pub async fn insert_dependency_into_redis(
        &mut self,
        namespace: String,
        name: String,
        version: String,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("start insert dependency");
        let key = format!("dependency:{namespace}:{name}:{version}");

        // 使用管道设置值和过期时间
        let _: () = redis::pipe()
            .cmd("SET")
            .arg(&key)
            .arg(&value)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(7 * 24 * 60 * 60) // 一周的秒数
            .query(&mut self.connection)?;
        println!("finish insert dependency");
        Ok(())
    }
    pub async fn insert_dependency_graph_into_redis(
        &mut self,
        namespace: String,
        name: String,
        version: String,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("start insert dependencygraph");
        let key = format!("dependencygraph:{namespace}:{name}:{version}");

        // 使用管道设置值和过期时间
        let _: () = redis::pipe()
            .cmd("SET")
            .arg(&key)
            .arg(&value)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(7 * 24 * 60 * 60) // 一周的秒数
            .query(&mut self.connection)?;
        println!("finish insert dependencygraph");
        Ok(())
    }
    pub async fn insert_dependent_into_redis(
        &mut self,
        namespace: String,
        name: String,
        version: String,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("start insert dependent");
        let key = format!("dependent:{namespace}:{name}:{version}");

        // 使用管道设置值和过期时间
        let _: () = redis::pipe()
            .cmd("SET")
            .arg(&key)
            .arg(&value)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(7 * 24 * 60 * 60) // 一周的秒数
            .query(&mut self.connection)?;
        println!("finish insert dependent");
        Ok(())
    }
    pub async fn insert_versionpage_into_redis(
        &mut self,
        namespace: String,
        name: String,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("start insert versionpage");
        let key = format!("versionpage:{namespace}:{name}");

        // 使用管道设置值和过期时间
        let _: () = redis::pipe()
            .cmd("SET")
            .arg(&key)
            .arg(&value)
            .cmd("EXPIRE")
            .arg(&key)
            .arg(7 * 24 * 60 * 60) // 一周的秒数
            .query(&mut self.connection)?;
        println!("finish insert versionpage");
        Ok(())
    }
}
