use std::collections::{VecDeque};

use crate::event::Message;

pub struct EventCache {
    inner: VecDeque<Message>,
    flushed: bool,
    flusher_handle: i64,
    
}
