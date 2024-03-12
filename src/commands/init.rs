//!
//!
//!
//!
//!
use crate::{cli::Config, commands::init};
use clap::{ArgMatches, Args, Command, FromArgMatches};
use common::errors::MegaResult;

use gateway::init::{init_dir, InitOptions};

pub fn cli() -> Command {
    InitOptions::augment_args_for_update(
        Command::new("init").about("Initialize the directory structure "),
    )
}

#[tokio::main]
pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = InitOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    println!("{server_matchers:#?}");
    Ok(())
}

#[cfg(test)]
mod tests {}
