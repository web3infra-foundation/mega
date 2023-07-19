//!
//!
//!
//!
//!
mod https;
mod p2p;
mod ssh;

use clap::{ArgMatches, Command};

use crate::cli::Config;
use common::errors::MegaResult;

pub fn builtin() -> Vec<Command> {
    vec![https::cli(), ssh::cli(), p2p::cli()]
}

pub(crate) fn builtin_exec(cmd: &str) -> Option<fn(Config, &ArgMatches) -> MegaResult> {
    let f = match cmd {
        "https" => https::exec,
        "ssh" => ssh::exec,
        "p2p" => p2p::exec,
        _ => return None,
    };

    Some(f)
}

#[cfg(test)]
mod tests {}
