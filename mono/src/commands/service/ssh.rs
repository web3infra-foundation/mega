use clap::{ArgMatches, Args, Command, FromArgMatches};
use context::AppContext;
use crate::{server::ssh_server::{start_server, SshOptions}};
use common::errors::MegaResult;

pub fn cli() -> Command {
    SshOptions::augment_args_for_update(Command::new("ssh").about("Start Git SSH server"))
}

pub(crate) async fn exec(ctx: AppContext, args: &ArgMatches) -> MegaResult {
    let server_matchers = SshOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    tracing::info!("{server_matchers:#?}");
    start_server(ctx, &server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
