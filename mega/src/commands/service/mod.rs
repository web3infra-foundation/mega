//!
//!
//!
//!
//!

use clap::{ArgMatches, Command};

use common::{config::Config, errors::MegaResult};

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
pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let (cmd, subcommand_args) = match args.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            return Ok(());
        }
    };
    match cmd {
        "https" => https::exec(config, subcommand_args).await,
        "ssh" => ssh::exec(config, subcommand_args).await,
        "start" => start::exec(config, subcommand_args).await,
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {}
