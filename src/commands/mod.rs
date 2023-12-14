//!
//!
//!
//!
//!
mod init;
mod service;

use clap::{ArgMatches, Command};

use crate::cli::Config;
use common::errors::MegaResult;

pub fn builtin() -> Vec<Command> {
    vec![
        init::cli(),
        service::cli(),
    ]
}

pub(crate) fn builtin_exec(cmd: &str) -> Option<fn(Config, &ArgMatches) -> MegaResult> {
    let f = match cmd {
        "init" => init::exec,
        "service" => service::exec,
        _ => return None,
    };

    Some(f)
}

#[cfg(test)]
mod tests {}
