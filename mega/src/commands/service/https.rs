use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::{config::Config, errors::MegaResult};
use gateway::https_server::{self, HttpOptions};


pub fn cli() -> Command {
    HttpOptions::augment_args_for_update(Command::new("https").about("Start Git HTTPS server"))
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = HttpOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");
    https_server::start_server(config, &server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
