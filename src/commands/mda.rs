//!
//!
//!
//!
//!
use clap::{ArgMatches, Args, Command, FromArgMatches};

use crate::cli::Config;
use common::errors::MegaResult;

use mda::run_mda;
pub fn cli() -> Command {
    run_mda::MDAOptions::augment_args_for_update(Command::new("mda").about("Start mda node"))
}

#[tokio::main]
pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = run_mda::MDAOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    // println!("{server_matchers:#?}");
    run_mda::run(server_matchers).unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}
