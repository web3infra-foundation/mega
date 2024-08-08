use std::fmt::Debug;
use std::sync::{Arc, OnceLock};

use chrono::Utc;
use crossbeam_channel::{unbounded, Sender};
use crossbeam_channel::Receiver;
use jupiter::context::Context;
use tokio::runtime::{Builder, Runtime};

use crate::cache::get_mcache;
use crate::event::{Message, EventType};

// Lazy initialized static MessageQueue instance.
pub(crate) static MQ: OnceLock<MessageQueue> = OnceLock::new();
pub fn get_mq() -> &'static MessageQueue {
    MQ.get().unwrap()
}

pub struct MessageQueue {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    // sem: Arc<Semaphore>,
    runtime: Arc<Runtime>,
    pub(crate) context: Context,
}

unsafe impl Send for MessageQueue{}
unsafe impl Sync for MessageQueue{}

impl Debug for MessageQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Just ignore context field.
        f.debug_struct("MessageQueue").field("sender", &self.sender).field("receiver", &self.receiver).field("runtime", &self.runtime).finish()
    }
}

impl MessageQueue {
    // Should be singleton.
    pub(crate) fn new(n_workers: usize, ctx: Context) -> Self {
        let (s, r) = unbounded::<Message>();
        let rt = Builder::new_multi_thread()
            .worker_threads(n_workers)
            .build()
            .unwrap();

        MessageQueue {
            sender: s.to_owned(),
            receiver: r.to_owned(),
            // sem: Arc::new(Semaphore::new(n_workers)),
            runtime: Arc::new(rt),
            context: ctx,
        }
    }

    pub(crate) fn start(&self) {
        let receiver = self.receiver.clone();
        // let sem = self.sem.clone();
        let rt = self.runtime.clone();

        tokio::spawn(async move {
            let mc = get_mcache();
            loop {
                match receiver.recv() {
                    Ok(evt) => {
                        let stored = evt.clone();
                        mc.add(stored).await;
                        rt.spawn(async move {
                            tracing::info!("{}", evt);
                        });
                    },
                    Err(e) => {
                        // Should not error here.
                        panic!("Event Loop Panic: {e}");
                    }
                }
            }
        });
    }

    pub(crate) fn send(&self, evt: EventType) {
        let _ = self.sender.send(Message {
            id: 1,
            create_time: Utc::now(),
            evt
        });
    }
}
