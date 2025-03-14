//! This module is responsible for handling the 'service' command.
//! It includes subcommands for starting different kinds of servers, such as HTTPS and SSH.
//!
//!
//!

use clap::{ArgMatches, Command};
use std::sync::Arc;

use common::{config::Config, errors::MegaResult};
use jupiter::context::Context;
use taurus::init::init_mq;

mod http;
mod https;
mod multi;
mod ssh;

// This function generates the CLI for the 'service' command.
// It includes subcommands for each server type.
pub fn cli() -> Command {
    let subcommands = vec![http::cli(), https::cli(), ssh::cli(), multi::cli()];
    Command::new("service")
        .about("Start different kinds of server: for example https or ssh")
        .subcommands(subcommands)
}

// This function executes the 'service' command.
// It determines which subcommand was used and calls the appropriate function.
#[tokio::main]
pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let config = Arc::new(config);
    let context = Context::new(config.clone()).await;
    init_mq(context.clone()).await;

    context
        .services
        .mono_storage
        .init_monorepo(&config.monorepo)
        .await;

    let (cmd, subcommand_args) = match args.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            return Ok(());
        }
    };
    match cmd {
        "http" => http::exec(context, subcommand_args).await,
        "https" => https::exec(context, subcommand_args).await,
        "ssh" => ssh::exec(context, subcommand_args).await,
        "multi" => multi::exec(context, subcommand_args).await,
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {}
