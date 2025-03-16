use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::errors::MegaResult;
use gateway::https_server::{self, HttpOptions};
use jupiter::context::Context;

pub fn cli() -> Command {
    HttpOptions::augment_args_for_update(Command::new("http").about("Start Mega HTTP server"))
}

pub(crate) async fn exec(context: Context, args: &ArgMatches) -> MegaResult {
    let server_matchers = HttpOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");
    https_server::http_server(context, server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
