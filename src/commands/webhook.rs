//!
//!
//!
//!
//!
use clap::{ArgMatches, Args, Command, FromArgMatches};

use crate::{cli::Config, commands::webhook};
use common::errors::MegaResult;

use gateway::webhook::{webhook_server, WebhookOptions};

pub fn cli() -> Command {
    WebhookOptions::augment_args_for_update(Command::new("webhook").about("Start github application webhook server"))
}

#[tokio::main]
pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = WebhookOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();
    println!("{server_matchers:#?}");
    webhook::webhook_server(&server_matchers).await.unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {}