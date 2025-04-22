//! Use the simplelog for logging.

extern crate dagrs;
extern crate log;
extern crate simplelog;

use std::collections::HashMap;

use dagrs::Dag;
use simplelog::*;

fn main() {
    // Initialize the global logger with a simplelogger as the logging backend.
    let _ = SimpleLogger::init(LevelFilter::Info, Config::default());

    let mut dag = Dag::with_yaml("tests/config/correct.yaml", HashMap::new()).unwrap();
    assert!(dag.start().is_ok());
}
