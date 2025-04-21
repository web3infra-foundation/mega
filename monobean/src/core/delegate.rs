use crate::application::Action;
use crate::core::mega_core::{MegaCommands, MegaCore};
use crate::core::runtime;
use async_channel::{Receiver, Sender};
use common::config::Config;
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
    pub fn new(action_sender: Sender<Action>, config: Config) -> MegaDelegate {
        static DELEGATE: OnceLock<MegaDelegate> = OnceLock::new();

        DELEGATE
            .get_or_init(|| {
                let (cmd_sender, cmd_receiver) = async_channel::unbounded();
                let ret = Self {
                    inner: cmd_sender.clone(),
                };

                ret.init_core(action_sender, cmd_receiver, config);
                ret
            })
            .clone()
    }

    fn init_core(
        &self,
        act_sender: Sender<Action>,
        cmd_receiver: Receiver<MegaCommands>,
        config: Config,
    ) {
        static CORE: OnceLock<MegaCore> = OnceLock::new();
        let core = CORE.get_or_init(move || MegaCore::new(act_sender, cmd_receiver, config));

        std::thread::spawn(move || {
            runtime().block_on(async move {
                core.init().await;
                while let Ok(cmd) = core.receiver.recv().await {
                    tokio::spawn(async move {
                        core.process_command(cmd).await;
                    });
                }
            });
        });
    }

    /// Send a command to the mega core and block
    pub async fn send_command(&self, cmd: MegaCommands) {
        let _ = self.inner.send(cmd).await;
    }

    /// Send a command to the mega core and block until the command is sent.
    ///
    /// # Deadlock
    ///
    /// Same as described in `MegaCore::process_command` and async_channel::Sender::send_blocking.
    /// Better not use this function in async context, or it would cause a deadlock.
    pub fn blocking_send_command(&self, cmd: MegaCommands) {
        let _ = self.inner.send_blocking(cmd);
    }
}
