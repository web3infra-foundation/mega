use std::path::PathBuf;

use clap::{ArgMatches, Args, Command, FromArgMatches, ValueEnum};

use common::{
    config::Config,
    errors::MegaResult,
    model::{CommonOptions, ZtmOptions},
};
use gateway::https_server::{self, HttpOptions, HttpsOptions};
use jupiter::context::Context;
use mono::server::ssh_server::{self, SshCustom, SshOptions};

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
    pub common: CommonOptions,

    #[clap(flatten)]
    pub ztm: ZtmOptions,

    #[arg(long, default_value_t = 8000)]
    pub http_port: u16,

    #[arg(long, default_value_t = 443)]
    pub https_port: u16,

    #[arg(long, value_name = "FILE")]
    https_key_path: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    https_cert_path: Option<PathBuf>,

    #[clap(flatten)]
    pub ssh: SshCustom,
}

pub fn cli() -> Command {
    StartOptions::augment_args_for_update(
        Command::new("multi").about("Start multiple server by given params"),
    )
}

pub(crate) async fn exec(config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = StartOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    tracing::info!("{server_matchers:#?}");

    let service_type = server_matchers.service;

    let context = Context::new(config.clone()).await;
    context
        .services
        .mono_storage
        .init_monorepo(&config.monorepo)
        .await;

    let context_clone = context.clone();
    let http_server = if service_type.contains(&StartCommand::Http) {
        let http = HttpOptions {
            common: server_matchers.common.clone(),
            http_port: server_matchers.http_port,
            ztm: server_matchers.ztm,
        };
        tokio::spawn(async move { https_server::http_server(context_clone, http).await })
    } else if service_type.contains(&StartCommand::Https) {
        let https = HttpsOptions {
            common: server_matchers.common.clone(),
            https_port: server_matchers.https_port,
            https_key_path: server_matchers.https_key_path.unwrap(),
            https_cert_path: server_matchers.https_cert_path.unwrap(),
            ztm: server_matchers.ztm,
        };
        tokio::spawn(async move { https_server::https_server(context_clone, https).await })
    } else {
        tokio::task::spawn(async {})
    };

    let ssh_server = if service_type.contains(&StartCommand::Ssh) {
        let ssh = SshOptions {
            common: server_matchers.common.clone(),
            custom: server_matchers.ssh,
        };
        tokio::spawn(async move { ssh_server::start_server(context, &ssh).await })
    } else {
        tokio::task::spawn(async {})
    };

    let _ = tokio::join!(http_server, ssh_server);

    Ok(())
}
