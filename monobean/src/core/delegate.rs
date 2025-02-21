use crate::application::Action;
use crate::core::mega_core::{MegaCommands, MegaCore};
use crate::core::runtime;
use async_channel::Sender;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

/// The delegate for the Mega core.
/// If we directly use mega core in the application,
/// it would be hard to operate cross glib and tokio runtimes.
/// So we use this struct to build up a channel and delegate the commands to the mega core.
pub struct MegaDelegate {
    inner: Sender<MegaCommands>,
    core: Arc<Mutex<MegaCore>>,
}

impl MegaDelegate {
    pub fn new(action_sender: Sender<Action>) -> &'static Self {
        static DELEGATE: OnceLock<MegaDelegate> = OnceLock::new();

        DELEGATE.get_or_init(move | |{
            let (cmd_sender, cmd_receiver) = async_channel::unbounded();
            let core = Arc::new(Mutex::new(MegaCore::new(action_sender, cmd_receiver)));
            let ret = Self {
                inner: cmd_sender,
                core,
            };

            ret.init_core();
            ret
        })
    }

    pub fn init_core(&self) {
        let core = self.core.clone();
        std::thread::spawn(move || {
            runtime().block_on(async move {
                if let Ok(mut lock) = core.try_lock() {
                    tokio::select! {
                        Ok(cmd) = lock.receiver.recv() => {
                            lock.process_command(cmd).await;
                        }
                    }
                } else {
                    panic!("Failed to lock mega core.");
                }
            });
        });
    }

    pub fn send_command(&self, cmd: MegaCommands) {
        let _ = self.inner.send_blocking(cmd);
    }
}