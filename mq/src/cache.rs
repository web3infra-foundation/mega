use std::{borrow::BorrowMut, cell::RefCell, collections::VecDeque, sync::{atomic::AtomicBool, Arc, Mutex, MutexGuard, OnceLock}, time::Duration};

use chrono::Utc;

use crate::{event::Message, queue::{get_mq, MessageQueue}};

const FLUSH_INTERVAL: u64 = 10;

// Lazy initialized static MessageCache instance.
pub fn get_mcache() -> &'static MessageCache {
    static MQ: OnceLock<MessageCache> = OnceLock::new();
    MQ.get_or_init(|| {
        // FIXME: Temp value
        let mc = MessageCache::new();
        mc.start();

        mc
    })
}

// Automatically flush message cache into database
// eveny 10 seconds or 1024 message.
pub struct MessageCache {
    inner: Arc<Mutex<Vec<Message>>>,
    bound_mq: &'static MessageQueue,
    last_flush: i64,
    stop: Arc<AtomicBool>,
}

impl MessageCache {
    fn new() -> Self {
        let now: chrono::DateTime<Utc> = Utc::now();

        MessageCache {
            inner: Arc::new(Mutex::new(Vec::new())),
            bound_mq: get_mq(),
            last_flush: now.timestamp_millis(),
            stop: Arc::new(AtomicBool::new(false))
        }
    }

    fn start(&self) {
        let stop = self.stop.clone();
        let _ = tokio::spawn(async move {
            loop {
                if !stop.load(std::sync::atomic::Ordering::Acquire) {
                    return
                }
                tokio::time::sleep(Duration::from_secs(FLUSH_INTERVAL)).await;
                instant_flush().await;
            }
        });
    }

    async fn add(&self, msg: Message) {
        let inner = self.inner.clone();
        let mut locked  = inner.lock().unwrap();

        if locked.len() >= 1024 {
            instant_flush().await;
        }
        locked.push(msg);
    }
}

pub async fn instant_flush() {
    let c = get_mcache();
}
