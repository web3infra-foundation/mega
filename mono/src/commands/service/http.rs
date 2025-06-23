use crate::server::https_server::{self};
use clap::{ArgMatches, Args, Command, FromArgMatches};
use common::{errors::MegaResult, model::CommonHttpOptions};
use context::AppContext;

pub fn cli() -> Command {
    CommonHttpOptions::augment_args_for_update(Command::new("http").about("Start Mega HTTP server"))
}

pub(crate) async fn exec(ctx: AppContext, args: &ArgMatches) -> MegaResult {
    let server_matchers: CommonHttpOptions = CommonHttpOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");
    https_server::start_http(ctx, server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
