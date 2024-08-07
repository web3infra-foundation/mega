use std::{collections::VecDeque, sync::OnceLock};

use crate::event::{Message, EventType};

// Lazy initialized static EventCache instance.
pub fn get_mcache() -> &'static EventCache {
    static MQ: OnceLock<EventCache> = OnceLock::new();
    MQ.get_or_init(|| {
        // FIXME: Temp value
        let mq = EventCache::new();

        mq
    })
}

// Automatically flush event cache into database
// eveny 10 seconds or 1024 message.
pub struct EventCache {
    inner: VecDeque<EventType>,
    last_flush: u64,
    flusher_handle: i64,
}

impl EventCache {
    fn new() -> Self {
        EventCache {
            inner: VecDeque::new(),
            last_flush: 0,
            flusher_handle: -1
        }
    }

    async fn flush(&self) {
        let v = vec![1,2,3,4];


    }
}
