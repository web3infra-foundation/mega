//! Cli module is responsible for parsing command line arguments and executing the appropriate.

use std::env;

use clap::{Arg, ArgMatches, Command};
use tracing_subscriber::fmt::writer::MakeWriterExt;

use common::{
    config::{Config, LogConfig},
    errors::{MegaError, MegaResult},
};

use crate::commands::{builtin, builtin_exec};

pub fn parse() -> MegaResult {
    let matches = cli().try_get_matches().unwrap_or_else(|e| e.exit());

    let current_dir = env::current_dir()?;
    let config_path = current_dir.join("config.toml");

    let config = if let Some(path) = matches.get_one::<String>("config").cloned() {
        Config::new(path.as_str()).unwrap()
    } else if config_path.exists() {
        Config::new(config_path.to_str().unwrap()).unwrap()
    } else {
        eprintln!("can't find config.toml under {:?}, you can manually set config.toml path with --config parameter", env::current_dir().unwrap());
        Config::default()
    };

    init_log(&config.log);

    let (cmd, subcommand_args) = match matches.subcommand() {
        Some((cmd, args)) => (cmd, args),
        _ => {
            // No subcommand provided
            // TODO: print some helping message to developer
            return Ok(());
        }
    };

    // TODO: match subcommand_args for `MegaResult`
    exec_subcommand(config, cmd, subcommand_args)
}

fn init_log(config: &LogConfig) {
    let log_level = match config.level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    let file_appender = tracing_appender::rolling::hourly(config.log_path.clone(), "mega-logs");

    if config.print_std {
        let stdout = std::io::stdout;
        tracing_subscriber::fmt()
            .with_writer(stdout.and(file_appender))
            .with_max_level(log_level)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_max_level(log_level)
            .init();
    }
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

#[cfg(test)]
mod tests {}
