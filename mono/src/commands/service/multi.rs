use clap::{ArgMatches, Args, Command, FromArgMatches, ValueEnum};

use crate::server::{
    http_server::{self},
    ssh_server::{self, SshCustom, SshOptions},
};
use common::{errors::MegaResult, model::CommonHttpOptions};
use context::AppContext;

#[derive(Debug, PartialEq, Clone, ValueEnum)]
pub enum StartCommand {
    Http,
    Https,
    Ssh,
}

#[derive(Args, Clone, Debug)]
pub struct StartOptions {
    service: Vec<StartCommand>,

    #[clap(flatten)]
    pub http: CommonHttpOptions,

    #[clap(flatten)]
    pub ssh: SshCustom,
}

pub fn cli() -> Command {
    StartOptions::augment_args_for_update(
        Command::new("multi").about("Start multiple server by given params"),
    )
}

pub(crate) async fn exec(ctx: AppContext, args: &ArgMatches) -> MegaResult {
    let server_matchers = StartOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");

    let service_type = server_matchers.service;

    let context_clone = ctx.clone();
    let http_server = if service_type.contains(&StartCommand::Http) {
        let http = server_matchers.http.clone();
        tokio::spawn(async move { http_server::start_http(context_clone, http).await })
    } else {
        panic!("start params should provide! run like 'mega service multi http ssh'")
    };

    let ssh_server = if service_type.contains(&StartCommand::Ssh) {
        let ssh = SshOptions {
            common: server_matchers.http.clone(),
            custom: server_matchers.ssh,
        };
        tokio::spawn(async move { ssh_server::start_server(ctx, &ssh).await })
    } else {
        tokio::task::spawn(async {})
    };

    let _ = tokio::join!(http_server, ssh_server);

    Ok(())
}
