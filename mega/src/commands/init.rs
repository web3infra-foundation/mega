//!
//!
//!
//!

use clap::{ArgMatches, Command};

use common::{config::Config, errors::MegaResult};
use gateway::init::init_monorepo;

pub fn cli() -> Command {
    Command::new("init").about("Initialize the mega monorepo structure")
}

#[tokio::main]
pub(crate) async fn exec(config: Config, _: &ArgMatches) -> MegaResult {
    init_monorepo(config).await.unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}
