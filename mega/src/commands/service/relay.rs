use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::{config::Config, errors::MegaResult};
use gateway::relay_server::{self, RelayOptions};

pub fn cli() -> Command {
    RelayOptions::augment_args_for_update(Command::new("relay").about("Start Mega RELAY server"))
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let relay_matchers = RelayOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{relay_matchers:#?}");
    relay_server::http_server(config, relay_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
