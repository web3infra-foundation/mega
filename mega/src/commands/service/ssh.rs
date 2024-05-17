use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::config::Config;
use common::errors::MegaResult;
use gateway::ssh_server::start_server;
use gateway::ssh_server::SshOptions;

use crate::commands::service::ssh;

pub fn cli() -> Command {
    SshOptions::augment_args_for_update(Command::new("ssh").about("Start Git SSH server"))
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = SshOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    tracing::info!("{server_matchers:#?}");
    ssh::start_server(config, &server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
