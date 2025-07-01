use clap::{ArgMatches, Args, Command, FromArgMatches, ValueEnum};

use common::{
    errors::MegaResult,
    model::{CommonHttpOptions, P2pOptions},
};
use context::AppContext;
use gateway::https_server::{self, HttpOptions};
use mono::server::ssh_server::{self, SshCustom, SshOptions};

#[derive(Debug, PartialEq, Clone, ValueEnum)]
pub enum StartCommand {
    Http,
    Ssh,
}

#[derive(Args, Clone, Debug)]
pub struct StartOptions {
    service: Vec<StartCommand>,

    #[clap(flatten)]
    pub http: CommonHttpOptions,

    #[clap(flatten)]
    pub p2p: P2pOptions,

    #[clap(flatten)]
    pub ssh: SshCustom,
}

pub fn cli() -> Command {
    StartOptions::augment_args_for_update(
        Command::new("multi").about("Start multiple server by given params"),
    )
}

pub(crate) async fn exec(context: AppContext, args: &ArgMatches) -> MegaResult {
    let server_matchers = StartOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");

    let service_type = server_matchers.service;

    let context_clone = context.clone();
    let http_server = if service_type.contains(&StartCommand::Http) {
        let http = HttpOptions {
            common: server_matchers.http.clone(),
            p2p: server_matchers.p2p,
        };
        tokio::spawn(async move { https_server::http_server(context_clone, http).await })
    } else {
        tokio::task::spawn(async {})
    };

    let ssh_server = if service_type.contains(&StartCommand::Ssh) {
        let ssh = SshOptions {
            common: server_matchers.http.clone(),
            custom: server_matchers.ssh,
        };
        tokio::spawn(async move { ssh_server::start_server(context, &ssh).await })
    } else {
        tokio::task::spawn(async {})
    };

    let _ = tokio::join!(http_server, ssh_server);

    Ok(())
}
