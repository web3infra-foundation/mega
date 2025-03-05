use crate::application::Action;
use crate::core::mega_core::{MegaCommands, MegaCore};
use crate::core::runtime;
use async_channel::{Receiver, Sender};
use std::sync::OnceLock;

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
            let ret = Self {
                inner: cmd_sender
            };

            ret.init_core(action_sender, cmd_receiver);
            ret
        })
    }

    fn init_core(&self, act_sender: Sender<Action>, cmd_receiver: Receiver<MegaCommands>) {
        static CORE: OnceLock<MegaCore> = OnceLock::new();
        let core = CORE.get_or_init(move || {
            MegaCore::new(act_sender, cmd_receiver)
        });

        std::thread::spawn(move || {
            runtime().block_on(async move {
                while let Ok(cmd) = core.receiver.recv().await {
                    tokio::spawn(async move {
                        core.process_command(cmd).await;
                    });
                }
            });
        });
    }

    pub async fn send_command(&self, cmd: MegaCommands) {
        let _ = self.inner.send(cmd).await;
    }
}