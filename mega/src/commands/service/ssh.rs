use clap::{ArgMatches, Args, Command, FromArgMatches};

use common::errors::MegaResult;
use context::AppContext;
use mono::server::ssh_server::SshOptions;
use mono::server::ssh_server::start_server;

pub fn cli() -> Command {
    SshOptions::augment_args_for_update(Command::new("ssh").about("Start Git SSH server"))
}

pub(crate) async fn exec(context: AppContext, args: &ArgMatches) -> MegaResult {
    let server_matchers = SshOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    tracing::info!("{server_matchers:#?}");
    start_server(context, &server_matchers).await;
    Ok(())
}

#[cfg(test)]
mod tests {}
