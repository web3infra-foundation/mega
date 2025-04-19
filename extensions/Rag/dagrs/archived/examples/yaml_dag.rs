//! Read the task information configured in the yaml file.

extern crate dagrs;

use dagrs::task::Content;
use dagrs::utils::file::load_file;
use dagrs::Dag;
use std::collections::HashMap;

fn main() {
    env_logger::init();
    let mut job = Dag::with_yaml("tests/config/correct.yaml", HashMap::new()).unwrap();
    assert!(job.start().is_ok());

    let content = load_file("tests/config/correct.yaml").unwrap();
    let mut job = Dag::with_yaml_str(&content, HashMap::new()).unwrap();
    assert!(job.start().is_ok());
    let out = job.get_results::<Content>();
    for (k, v) in out {
        println!("{k} {:#?}", v.unwrap().get::<(Vec<String>, Vec<String>)>());
    }
}
