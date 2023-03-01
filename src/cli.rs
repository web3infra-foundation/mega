//!
//!
//!
//!
//!
use clap::{Arg, ArgMatches, Command};
use config as c;
use serde::Deserialize;

use crate::errors::{MegaError, MegaResult};
use crate::commands::{builtin, builtin_exec};

#[derive(Debug, Deserialize)]
pub(crate) struct Config {}

impl Config {
    pub fn new(path: &str) -> Result<Self, c::ConfigError> {
        let builder = c::Config::builder()
            .add_source(c::File::new(path,
                                     c::FileFormat::Toml));
        let config = builder.build().unwrap();

        Config::from_config(&config)
    }

    pub fn from_config(config: &c::Config) -> Result<Self, c::ConfigError> {
        config.get::<Self>(env!("CARGO_PKG_NAME"))
    }

    pub fn default() -> Self {
        Config {}
    }
}

pub fn parse() -> MegaResult{
    let matches = cli().try_get_matches().unwrap_or_else(|e| e.exit());
    let mut config = Config::default();

    if let Some(c) = matches.get_one::<String>("config").cloned() {
        config = Config::new(c.as_str()).unwrap()
    }

    let (cmd, subcommand_args) = match matches.subcommand() {
      Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided.
            return Ok(());
        }
    };

    exec_subcommand(config, cmd, subcommand_args)
}

fn cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommands(builtin())
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Sets a config file work directory"),
        )
}

fn exec_subcommand(config: Config, cmd: &str, args: &ArgMatches) -> MegaResult {
    if let Some(f) = builtin_exec(cmd) {
        f(config, args)
    } else {
        Err(MegaError::unknown_subcommand(cmd))
    }
}
