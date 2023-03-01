//!
//!
//!
//!
//!
use clap::{ArgMatches, Command};

use crate::errors::MegaResult;
use crate::cli::Config;

pub fn cli() -> Command {
    Command::new("https")
        .about("Start Git HTTPS server")
}

pub(crate) fn exec(_config: Config, _args: &ArgMatches) -> MegaResult {
    Ok(())
}