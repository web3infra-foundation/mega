use std::sync::{Arc, OnceLock};

use chrono::Utc;
use crossbeam_channel::{unbounded, Sender};
use crossbeam_channel::Receiver;
use tokio::runtime::{Builder, Runtime};

use crate::event::{EventBase, Message, EventType};

// Lazy initialized static MessageQueue instance.
pub fn get_mq() -> &'static MessageQueue {
    static MQ: OnceLock<MessageQueue> = OnceLock::new();
    MQ.get_or_init(|| {
        // FIXME: Temp value
        let mq = MessageQueue::new(12);
        mq.start();

        mq
    })
}

pub struct MessageQueue {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    // sem: Arc<Semaphore>,
    runtime: Arc<Runtime>,
}

unsafe impl Send for MessageQueue{}
unsafe impl Sync for MessageQueue{}

impl MessageQueue {
    // Should be singleton.
    fn new(n_workers: usize) -> Self {
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
        }
    }

    fn start(&self) {
        let receiver = self.receiver.clone();
        // let sem = self.sem.clone();
        let rt = self.runtime.clone();

        tokio::spawn(async move {
            loop {
                match receiver.recv() {
                    Ok(evt) => {
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

    pub fn send(&self, evt: EventType) {
        let _ = self.sender.send(Message {
            id: 1,
            create_time: Utc::now(),
            evt
        });
    }
}
