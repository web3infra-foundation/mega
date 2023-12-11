//!
//!
//!
//!
//!
use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::errors::MegaResult;
use gateway::ssh::server;
use gateway::ssh::SshOptions;

use crate::cli::Config;
use crate::commands::server::ssh;

pub fn cli() -> Command {
    SshOptions::augment_args_for_update(Command::new("ssh").about("Start Git SSH server"))
}

#[tokio::main]
pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = SshOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    println!("{server_matchers:#?}");
    ssh::server(&server_matchers).await.unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}
