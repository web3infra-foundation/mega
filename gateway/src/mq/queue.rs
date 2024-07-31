use std::{borrow::Borrow, sync::Arc};

use crossbeam_channel::{unbounded, Receiver, Sender};
use tokio::{
    runtime::{Builder, Runtime}, select, sync::Semaphore
};

use super::event::{EventType, Message};

pub(crate) struct MessageQueue {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    sem: Arc<Semaphore>,
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
            sem: Arc::new(Semaphore::new(n_workers)),
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
                            let evt = evt.clone();
                            evt.process().await;
                            evt.done().await;
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

    pub fn enqueue(&self, msg: Message) {
        match self.sender.send(msg) {
            Ok(()) => {}
            Err(_) => {}
        }
    }
}

pub(crate) fn init_message_queue(n_workers: usize) -> MessageQueue {
    let mq = MessageQueue::new(n_workers);
    mq.start();

    mq
}
