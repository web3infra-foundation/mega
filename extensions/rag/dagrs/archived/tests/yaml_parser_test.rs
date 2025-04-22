use std::collections::HashMap;

use dagrs::{DagError, Parser, Task, YamlParser};

#[test]
fn file_not_found_test() {
    let no_such_file: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("./no_such_file.yaml", HashMap::new());
    // let err = no_such_file.unwrap_err().to_string();
    // println!("{err}");
    assert!(no_such_file.is_err())
}

#[test]
fn illegal_yaml_content() {
    let illegal_content: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("tests/config/illegal_content.yaml", HashMap::new());
    // let err = illegal_content.unwrap_err().to_string();
    // println!("{err}");
    assert!(illegal_content.is_err())
}

#[test]
fn empty_content() {
    let empty_content: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("tests/config/empty_file.yaml", HashMap::new());
    // let err = empty_content.unwrap_err().to_string();
    // println!("{err}");
    assert!(empty_content.is_err())
}

#[test]
fn yaml_no_start_with_dagrs() {
    let forget_dagrs: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("tests/config/no_start_with_dagrs.yaml", HashMap::new());
    // let err = forget_dagrs.unwrap_err().to_string();
    // println!("{err}");
    assert!(forget_dagrs.is_err())
}

#[test]
fn yaml_task_no_name() {
    let no_task_name: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("tests/config/no_task_name.yaml", HashMap::new());
    // let err = no_task_name.unwrap_err().to_string();
    // println!("{err}");
    assert!(no_task_name.is_err())
}

#[test]
fn yaml_task_not_found_precursor() {
    let not_found_pre: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("tests/config/precursor_not_found.yaml", HashMap::new());
    // let err = not_found_pre.unwrap_err().to_string();
    // println!("{err}");
    assert!(not_found_pre.is_err())
}

#[test]
fn yaml_task_no_script_config() {
    let script: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("tests/config/no_script.yaml", HashMap::new());
    // let err = script.unwrap_err().to_string();
    // println!("{err}");
    assert!(script.is_err())
}

#[test]
fn correct_parse() {
    let tasks: Result<Vec<Box<dyn Task>>, DagError> =
        YamlParser.parse_tasks("tests/config/correct.yaml", HashMap::new());
    assert!(tasks.is_ok());
}
