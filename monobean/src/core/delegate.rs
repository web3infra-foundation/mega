use crate::application::Action;
use crate::core::mega_core::{MegaCommands, MegaCore};
use crate::core::runtime;
use async_channel::Sender;
use futures::StreamExt;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

/// The delegate for the Mega core.
/// If we directly use mega core in the application,
/// it would be hard to operate cross glib and tokio runtimes.
/// So we use this struct to build up a channel and delegate the commands to the mega core.
#[derive(Clone, Debug)]
pub struct MegaDelegate {
    inner: Sender<MegaCommands>,
}

impl MegaDelegate {
    pub fn new(action_sender: Sender<Action>) -> &'static Self {
        static DELEGATE: OnceLock<MegaDelegate> = OnceLock::new();

        DELEGATE.get_or_init(move || {
            let (cmd_sender, cmd_receiver) = async_channel::unbounded();
            let core = Arc::new(Mutex::new(MegaCore::new(action_sender, cmd_receiver)));
            let ret = Self {
                inner: cmd_sender
            };

            ret.init_core(core);
            ret
        })
    }

    fn init_core(&self, core: Arc<Mutex<MegaCore>>) {
        std::thread::spawn(move || {
            runtime().block_on(async move {
                if let Ok(mut lock) = core.clone().try_lock() {
                    while let Ok(cmd) = lock.receiver.recv().await {
                        lock.process_command(cmd).await;
                        tracing::debug!("Processing done: {}", lock.receiver.is_empty());
                    }
                } else {
                    panic!("Failed to lock mega core.");
                }
            });
        });
    }

    pub async fn send_command(&self, cmd: MegaCommands) {
        let _ = self.inner.send(cmd).await;
    }
}