//!
//!
//!
//!
//!
use clap::{ArgMatches, Command};

use common::errors::MegaResult;

use crate::cli::Config;

mod https;
mod p2p;
mod ssh;

pub fn cli() -> Command {
    let subcommands = vec![https::cli(), ssh::cli(), p2p::cli()];
    Command::new("service")
        .about("Start different kinds of server: for example https, ssh, p2p")
        .subcommands(subcommands)
}

pub(crate) fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let (cmd, subcommand_args) = match args.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            return Ok(());
        }
    };
    match cmd {
        "https" => https::exec(_config, subcommand_args),
        "ssh" => ssh::exec(_config, subcommand_args),
        "p2p" => p2p::exec(_config, subcommand_args),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {}
