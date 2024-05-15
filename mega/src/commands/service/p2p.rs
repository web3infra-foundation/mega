use clap::{ArgMatches, Args, Command, FromArgMatches};

use crate::cli::Config;
use common::errors::MegaResult;

use p2p::peer;

pub fn cli() -> Command {
    peer::P2pOptions::augment_args_for_update(Command::new("p2p").about("Start p2p node"))
}

pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = peer::P2pOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    trace::info!("{server_matchers:#?}");
    peer::run(&server_matchers).await.unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}
