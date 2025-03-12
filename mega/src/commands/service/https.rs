use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::{config::Config, errors::MegaResult};
use gateway::https_server::{self, HttpsOptions};
use jupiter::context::Context;

pub fn cli() -> Command {
    HttpsOptions::augment_args_for_update(Command::new("https").about("Start Mega HTTPS server"))
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = HttpsOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");
    let context = Context::new(config.clone()).await;
    context
        .services
        .mono_storage
        .init_monorepo(&config.monorepo)
        .await;
    https_server::https_server(context, server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
