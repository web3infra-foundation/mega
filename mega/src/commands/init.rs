//! This module is responsible for handling the 'init' command.
//! It initializes the mega monorepo structure.
//!
//!

use clap::{ArgMatches, Command};

use common::{config::Config, errors::MegaResult};
use gateway::init::init_monorepo;

// This function generates the CLI for the 'init' command.
pub fn cli() -> Command {
    Command::new("init").about("Initialize the mega monorepo structure")
}

// This function executes the 'init' command.
// It calls the `init_monorepo` function from the `gateway` module.
#[tokio::main]
pub(crate) async fn exec(config: Config, _: &ArgMatches) -> MegaResult {
    init_monorepo(config).await.unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}
