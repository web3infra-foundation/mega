use clap::{ArgMatches, Args, Command, FromArgMatches};

use crate::server::ssh_server::{start_server, SshOptions};
use common::config::Config;
use common::errors::MegaResult;
use jupiter::context::Context;

pub fn cli() -> Command {
    SshOptions::augment_args_for_update(Command::new("ssh").about("Start Git SSH server"))
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = SshOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    tracing::info!("{server_matchers:#?}");
    let context = Context::new(config.clone()).await;
    context
        .services
        .mono_storage
        .init_monorepo(&config.monorepo)
        .await;
    start_server(context, &server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
