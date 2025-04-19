use std::{collections::HashMap, fs::File, str::FromStr};

use clap::Parser;
use dagrs::Dag;

#[derive(Parser, Debug)]
#[command(name = "dagrs", version = "0.2.0")]
struct Args {
    /// Log output file, the default is to print to the terminal.
    #[arg(long)]
    log_path: Option<String>,
    /// yaml configuration file path.
    #[arg(long)]
    yaml: String,
    /// Log level, the default is 'info'.
    #[arg(long)]
    log_level: Option<String>,
}

fn main() {
    let args = Args::parse();

    init_logger(&args);

    let yaml_path = args.yaml;
    let mut dag = Dag::with_yaml(yaml_path.as_str(), HashMap::new()).unwrap();
    assert!(dag.start().is_ok());
}

fn init_logger(args: &Args) {
    let log_level = match &args.log_level {
        Some(level_str) => log::LevelFilter::from_str(level_str).unwrap(),
        None => log::LevelFilter::Info,
    };
    let mut logger_builder = env_logger::Builder::new();
    logger_builder.filter_level(log_level);

    // initialize the env_logger with the given log_path
    if let Some(log_path) = &args.log_path {
        logger_builder.target(env_logger::Target::Pipe(Box::new(
            File::create(log_path).unwrap(),
        )));
    };

    logger_builder.init();
}
