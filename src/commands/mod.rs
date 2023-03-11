//!
//!
//!
//!
//!
mod https;

use clap::{ArgMatches, Command};

use crate::cli::Config;
use crate::errors::MegaResult;

pub fn builtin() -> Vec<Command> {
    vec![https::cli()]
}

pub(crate) fn builtin_exec(cmd: &str) -> Option<fn(Config, &ArgMatches) -> MegaResult> {
    let f = match cmd {
        "https" => https::exec,
        _ => return None,
    };

    Some(f)
}

#[cfg(test)]
mod tests {}