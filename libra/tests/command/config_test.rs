use libra::command::config;
use serial_test::serial;
use tempfile::tempdir;

use super::*;
#[tokio::test]
#[serial]
async fn test_config_get_failed() {
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    let args = config::ConfigArgs {
        add: true,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: Some("value".to_string()),
        default: Some("erasernoob".to_string()),
        name_only: false,
    };
    config::execute(args).await;
}

#[tokio::test]
#[serial]
async fn test_config_get_all() {
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    // set the current working directory to the temporary path
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    // Add the config first
    let arg1 = config::ConfigArgs {
        add: true,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: Some("erasernoob".to_string()),
        default: None,
        name_only: false,
    };
    config::execute(arg1).await;

    let args = config::ConfigArgs {
        add: false,
        get: true,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: None,
        default: None,
        name_only: false,
    };
    config::execute(args).await;
}

#[tokio::test]
#[serial]
async fn test_config_get_all_with_default() {
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    // set the current working directory to the temporary path
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let args = config::ConfigArgs {
        add: false,
        get: false,
        get_all: true,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: Some("value".to_string()),
        default: Some("erasernoob".to_string()),
        name_only: false,
    };
    config::execute(args).await;
}

#[tokio::test]
#[serial]
async fn test_config_get() {
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    // set the current working directory to the temporary path
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    // Add the config first
    let arg1 = config::ConfigArgs {
        add: true,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: Some("erasernoob".to_string()),
        default: None,
        name_only: false,
    };
    config::execute(arg1).await;

    let args = config::ConfigArgs {
        add: false,
        get: true,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: None,
        default: None,
        name_only: false,
    };
    config::execute(args).await;
}

#[tokio::test]
#[serial]
async fn test_config_get_with_default() {
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let args = config::ConfigArgs {
        add: false,
        get: true,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: None,
        default: Some("erasernoob".to_string()),
        name_only: false,
    };
    config::execute(args).await;
}

#[tokio::test]
#[serial]
async fn test_config_list() {
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    // set the current working directory to the temporary path
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    // Add the config first
    let arg1 = config::ConfigArgs {
        add: true,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: Some("erasernoob".to_string()),
        default: None,
        name_only: false,
    };
    config::execute(arg1).await;

    let arg2 = config::ConfigArgs {
        add: true,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.email".to_string()),
        valuepattern: Some("erasernoob@example.com".to_string()),
        default: None,
        name_only: false,
    };
    config::execute(arg2).await;

    // List configs
    let args = config::ConfigArgs {
        add: false,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: true,
        key: None,
        valuepattern: None,
        default: None,
        name_only: false,
    };
    assert!(args.validate().is_ok());
    config::execute(args).await;
}

#[tokio::test]
#[serial]
async fn test_config_list_name_only() {
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    // set the current working directory to the temporary path
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    // Add the config first
    let arg1 = config::ConfigArgs {
        add: true,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.name".to_string()),
        valuepattern: Some("erasernoob".to_string()),
        default: None,
        name_only: false,
    };
    config::execute(arg1).await;

    let arg2 = config::ConfigArgs {
        add: true,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: Some("user.email".to_string()),
        valuepattern: Some("erasernoob@example.com".to_string()),
        default: None,
        name_only: false,
    };
    config::execute(arg2).await;

    // List configs with name_only set to true
    let args = config::ConfigArgs {
        add: false,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: true,
        key: None,
        valuepattern: None,
        default: None,
        name_only: true,
    };
    assert!(args.validate().is_ok());
    config::execute(args).await;
}

#[tokio::test]
#[serial]
async fn test_config_list_name_only_without_list(){
    let temp_path = tempdir().unwrap();
    // start a new libra repository in a temporary directory
    test::setup_with_new_libra_in(temp_path.path()).await;

    // set the current working directory to the temporary path
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let args = config::ConfigArgs {
        add: false,
        get: false,
        get_all: false,
        unset: false,
        unset_all: false,
        list: false,
        key: None,
        valuepattern: None,
        default: None,
        name_only: true,
    };
    assert!(args.validate().is_err());
}
