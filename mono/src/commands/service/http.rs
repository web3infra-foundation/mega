use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::{config::Config, errors::MegaResult};
use crate::server::https_server::{self, HttpOptions};



pub fn cli() -> Command {
    HttpOptions::augment_args_for_update(Command::new("http").about("Start Mega HTTP server"))
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = HttpOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");
    https_server::start_http(config, server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
