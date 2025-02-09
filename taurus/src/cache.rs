use std::{mem::swap, sync::{atomic::{AtomicBool, AtomicI64}, Arc, Mutex, OnceLock}, time::Duration};

use chrono::Utc;

use crate::{event::Message, queue::{get_mq, MessageQueue}};

const FLUSH_INTERVAL: u64 = 10;

// Lazy initialized static MessageCache instance.
pub fn get_mcache() -> &'static MessageCache {
    static MC: OnceLock<MessageCache> = OnceLock::new();
    MC.get_or_init(|| {
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
    last_flush: Arc<AtomicI64>,
    stop: Arc<AtomicBool>,
}

impl MessageCache {
    fn new() -> Self {
        let now: chrono::DateTime<Utc> = Utc::now();

        MessageCache {
            inner: Arc::new(Mutex::new(Vec::new())),
            bound_mq: get_mq(),
            last_flush: Arc::new(AtomicI64::new(now.timestamp_millis())),
            stop: Arc::new(AtomicBool::new(false))
        }
    }

    fn start(&self) {
        let stop = self.stop.clone();
        tokio::spawn(async move {
            loop {
                if stop.load(std::sync::atomic::Ordering::Acquire) {
                    return
                }
                tokio::time::sleep(Duration::from_secs(FLUSH_INTERVAL)).await;

                instant_flush().await;
            }
        });
    }

    fn get_cache(&self) -> Vec<Message> {
        let mut res = Vec::new();
        let inner = self.inner.clone();

        let mut locked  = inner.lock().unwrap();
        if !locked.is_empty() {
            swap(locked.as_mut(), &mut res);
        }

        res
    }

    pub(crate) async fn add(&self, msg: Message) -> &Self {
        let inner = self.inner.clone();
        let should_flush: bool;
        {
            let mut locked  = inner.lock().unwrap();
            let l = locked.len();
            should_flush = l >= 1;
            locked.push(msg);
        }

        if should_flush {
            instant_flush().await
        }

        self
    }
}

pub async fn instant_flush() {
    use callisto::mq_storage::Model;

    let mc = get_mcache();
    let st = mc.bound_mq.context.services.mq_storage.clone();
    let data = mc
        .get_cache()
        .into_iter().map(Into::<Model>::into)
        .collect::<Vec<Model>>();
    st.save_messages(data).await;

    let now =  Utc::now();
    mc.last_flush.to_owned().store(now.timestamp_millis(), std::sync::atomic::Ordering::Relaxed);
}
