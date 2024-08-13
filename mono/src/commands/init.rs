//! This module is responsible for handling the 'init' command.
//! It initializes the monorepo structure.
//!
//!

use clap::{ArgMatches, Command};

use common::{config::Config, errors::MegaResult};
use jupiter::context::Context;

// This function generates the CLI for the 'init' command.
pub fn cli() -> Command {
    Command::new("init").about("Initialize the monorepo structure")
}

// This function executes the 'init' command.
// It calls the `init_monorepo` function from the `gateway` module.
#[tokio::main]
pub(crate) async fn exec(config: Config, _: &ArgMatches) -> MegaResult {

    let context = Context::new(config).await;
    context.services.mega_storage.init_monorepo().await;
    Ok(())
}

#[cfg(test)]
mod tests {}
