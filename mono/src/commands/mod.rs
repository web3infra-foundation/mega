pub mod service;

use clap::{ArgMatches, Command};
use common::{config::Config, errors::MegaResult};

pub fn builtin() -> Vec<Command> {
    vec![service::cli()]
}

pub(crate) fn builtin_exec(cmd: &str) -> Option<fn(Config, &ArgMatches) -> MegaResult> {
    let f = match cmd {
        "service" => service::exec,
        _ => return None,
    };

    Some(f)
}

#[cfg(test)]
mod tests {}
