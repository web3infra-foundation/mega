//!
//!
//!
//!
//!
mod https;
mod ssh;

use clap::{ArgMatches, Command};

use crate::cli::Config;
use common::errors::MegaResult;

pub fn builtin() -> Vec<Command> {
    vec![https::cli(), ssh::cli()]
}

pub(crate) fn builtin_exec(cmd: &str) -> Option<fn(Config, &ArgMatches) -> MegaResult> {
    let f = match cmd {
        "https" => https::exec,
        "ssh" => ssh::exec,
        _ => return None,
    };

    Some(f)
}

#[cfg(test)]
mod tests {}
