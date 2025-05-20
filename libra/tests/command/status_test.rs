use super::*;
use std::fs;
use std::io::Write;
#[tokio::test]
#[serial]
/// Tests the file status detection functionality with respect to ignore patterns.
/// Verifies that files matching patterns in .libraignore are properly excluded from status reports.
async fn test_changes_to_be_staged() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    let mut gitignore_file = fs::File::create(".libraignore").unwrap();
    gitignore_file
        .write_all(b"should_ignore*\nignore_dir/")
        .unwrap();

    let mut should_ignore_file_0 = fs::File::create("should_ignore.0").unwrap();
    let mut not_ignore_file_0 = fs::File::create("not_ignore.0").unwrap();
    fs::create_dir("ignore_dir").unwrap();
    let mut should_ignore_file_1 = fs::File::create("ignore_dir/should_ignore.1").unwrap();
    fs::create_dir("not_ignore_dir").unwrap();
    let mut not_ignore_file_1 = fs::File::create("not_ignore_dir/not_ignore.1").unwrap();

    let change = changes_to_be_staged();
    assert!(!change
        .new
        .iter()
        .any(|x| x.file_name().unwrap() == "should_ignore.0"));
    assert!(!change
        .new
        .iter()
        .any(|x| x.file_name().unwrap() == "should_ignore.1"));
    assert!(change
        .new
        .iter()
        .any(|x| x.file_name().unwrap() == "not_ignore.0"));
    assert!(change
        .new
        .iter()
        .any(|x| x.file_name().unwrap() == "not_ignore.1"));

    add::execute(AddArgs {
        pathspec: vec![String::from(".")],
        all: true,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    should_ignore_file_0.write_all(b"foo").unwrap();
    should_ignore_file_1.write_all(b"foo").unwrap();
    not_ignore_file_0.write_all(b"foo").unwrap();
    not_ignore_file_1.write_all(b"foo").unwrap();

    let change = changes_to_be_staged();
    assert!(!change
        .modified
        .iter()
        .any(|x| x.file_name().unwrap() == "should_ignore.0"));
    assert!(!change
        .modified
        .iter()
        .any(|x| x.file_name().unwrap() == "should_ignore.1"));
    assert!(change
        .modified
        .iter()
        .any(|x| x.file_name().unwrap() == "not_ignore.0"));
    assert!(change
        .modified
        .iter()
        .any(|x| x.file_name().unwrap() == "not_ignore.1"));

    fs::remove_dir_all("ignore_dir").unwrap();
    fs::remove_dir_all("not_ignore_dir").unwrap();
    fs::remove_file("should_ignore.0").unwrap();
    fs::remove_file("not_ignore.0").unwrap();

    not_ignore_file_1.write_all(b"foo").unwrap();

    let change = changes_to_be_staged();
    assert!(!change
        .deleted
        .iter()
        .any(|x| x.file_name().unwrap() == "should_ignore.0"));
    assert!(!change
        .deleted
        .iter()
        .any(|x| x.file_name().unwrap() == "should_ignore.1"));
    assert!(change
        .deleted
        .iter()
        .any(|x| x.file_name().unwrap() == "not_ignore.0"));
    assert!(change
        .deleted
        .iter()
        .any(|x| x.file_name().unwrap() == "not_ignore.1"));
}
