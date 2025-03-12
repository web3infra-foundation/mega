use clap::{ArgMatches, Args, Command, FromArgMatches};

use crate::server::https_server::{self, HttpOptions};
use common::{config::Config, errors::MegaResult};
use jupiter::context::Context;

pub fn cli() -> Command {
    HttpOptions::augment_args_for_update(Command::new("http").about("Start Mega HTTP server"))
}

pub(crate) async fn exec(ctx: Context, args: &ArgMatches) -> MegaResult {
    let server_matchers = HttpOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");
    https_server::start_http(ctx, server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
