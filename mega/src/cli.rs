//! Cli module is responsible for parsing command line arguments and executing the appropriate.

use clap::{Arg, ArgMatches, Command};
use std::env;
use std::path::PathBuf;
use tracing_subscriber::fmt::writer::MakeWriterExt;

use common::{
    config::{Config, LogConfig},
    errors::{MegaError, MegaResult},
};

use crate::commands::{builtin, builtin_exec};

/// This function is responsible for parsing command line arguments.
/// It uses the `cli` function to get the matches for the command line arguments.
/// If the matches are not found, it will exit the program.
///
/// # Returns
///
/// This function returns a `MegaResult`. If the parsing is successful, it will return the result.
/// If there is an error during the parsing, it will return an error.
pub fn parse(args: Option<Vec<&str>>) -> MegaResult {
    let matches = match args {
        Some(args) => cli()
            .no_binary_name(true)
            .try_get_matches_from(args)
            .unwrap_or_else(|e| e.exit()),
        None => cli().try_get_matches().unwrap_or_else(|e| e.exit()),
    };

    // Load configuration from the config file or default location
    let current_dir = env::current_dir()?;
    let base_dir = common::config::mega_base();
    let config_path = current_dir.join("config.toml");
    let config_path_alt = base_dir.join("etc/config.toml");

    let config = if let Some(path) = matches.get_one::<PathBuf>("config").cloned() {
        Config::new(path.to_str().unwrap()).unwrap()
    } else if config_path.exists() {
        Config::new(config_path.to_str().unwrap()).unwrap()
    } else if config_path_alt.exists() {
        Config::new(config_path_alt.to_str().unwrap()).unwrap()
    } else {
        eprintln!("can't find config.toml under {:?} or {:?}, you can manually set config.toml path with --config parameter", env::current_dir().unwrap(), base_dir);
        Config::default()
    };

    init_log(&config.log);

    ctrlc::set_handler(move || {
        tracing::info!("Received Ctrl-C signal, exiting...");
        std::process::exit(0);
    })
    .unwrap();

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
                .value_parser(clap::value_parser!(PathBuf))
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
