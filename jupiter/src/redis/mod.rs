pub mod lock;

pub fn new_client(redis_url: &str) -> redis::Client {
    redis::Client::open(redis_url).unwrap()
}

pub fn mock() -> redis::Client {
    redis::Client::open("redis://127.0.0.1:6379").unwrap()
}
