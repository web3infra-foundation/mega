//!
//!
//!
//!
//!
use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::errors::MegaResult;
use gateway::init::{init_monorepo, InitOptions};

use crate::cli::Config;

pub fn cli() -> Command {
    InitOptions::augment_args_for_update(
        Command::new("init").about("Initialize the mega monorepo structure"),
    )
}

#[tokio::main]
pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = InitOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    init_monorepo(&server_matchers).await.unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}
