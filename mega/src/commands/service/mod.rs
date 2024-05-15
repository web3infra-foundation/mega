use clap::{ArgMatches, Command};

use common::errors::MegaResult;

use crate::cli::Config;

mod https;
mod ssh;
mod start;

pub fn cli() -> Command {
    let subcommands = vec![https::cli(), ssh::cli(), start::cli()];
    Command::new("service")
        .about("Start different kinds of server: for example https or ssh")
        .subcommands(subcommands)
}

#[tokio::main]
pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let (cmd, subcommand_args) = match args.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            return Ok(());
        }
    };
    match cmd {
        "https" => https::exec(_config, subcommand_args).await,
        "ssh" => ssh::exec(_config, subcommand_args).await,
        "start" => start::exec(_config, subcommand_args).await,
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {}
