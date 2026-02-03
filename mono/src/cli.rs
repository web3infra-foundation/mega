//! Cli module is responsible for parsing command line arguments and executing the appropriate.

use std::{env, path::PathBuf};

use clap::{Arg, ArgMatches, Command};
use common::{
    config::{
        Config, LogConfig,
        loader::{ConfigInput, ConfigLoader},
    },
    errors::{MegaError, MegaResult},
};
use tracing_subscriber::fmt::writer::MakeWriterExt;

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

    let cli_path = matches.get_one::<PathBuf>("config").cloned();
    let input = ConfigInput {
        cli_path,
        env_path: std::env::var_os("MEGA_CONFIG").map(PathBuf::from),
    };
    let loaded = ConfigLoader::new(input).load()?;

    let config = Config::new(loaded.path.to_str().ok_or_else(|| {
        MegaError::Other(format!(
            "Config path contains invalid UTF-8: {:?}",
            loaded.path
        ))
    })?)?;

    init_log(&config.log);

    tracing::info!(
        source = ?loaded.source,
        path = %loaded.path.display(),
        "config loaded"
    );

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

    let file_appender = tracing_appender::rolling::hourly(config.log_path.clone(), "mono-logs");

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
        Err(MegaError::Other(format!("Unknown subcommand: {}", cmd)))
    }
}

#[cfg(test)]
mod tests {}
