use clap::{ArgMatches, Args, Command, FromArgMatches};

use crate::server::https_server::{self, HttpOptions};
use common::{config::Config, errors::MegaResult};
use jupiter::context::Context;

pub fn cli() -> Command {
    HttpOptions::augment_args_for_update(Command::new("http").about("Start Mega HTTP server"))
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = HttpOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");
    let context = Context::new(config.clone()).await;
    context
        .services
        .mono_storage
        .init_monorepo(&config.monorepo)
        .await;
    https_server::start_http(context, server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
