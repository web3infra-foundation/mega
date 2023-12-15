//!
//!
//!
//!
//!
use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::errors::MegaResult;
use gateway::https_server::{self, HttpOptions};

use crate::cli::Config;

pub fn cli() -> Command {
    HttpOptions::augment_args_for_update(Command::new("https").about("Start Git HTTPS server"))
}

pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = HttpOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    println!("{server_matchers:#?}");
    https_server::start_server(&server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
