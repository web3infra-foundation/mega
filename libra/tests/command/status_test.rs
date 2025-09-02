use super::*;
use std::fs;
use std::io::Write;
use libra::command::status::StatusArgs;
use libra::command::status::execute_to as status_execute;
use libra::command::status::output_porcelain;
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
        refresh: false,
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

#[test]
fn test_output_porcelain_format() {
    use libra::command::status::Changes;
    use std::path::PathBuf;
    
    // Create test data
    let staged = Changes {
        new: vec![PathBuf::from("new_file.txt")],
        modified: vec![PathBuf::from("modified_file.txt")],
        deleted: vec![PathBuf::from("deleted_file.txt")],
    };
    
    let unstaged = Changes {
        new: vec![PathBuf::from("untracked_file.txt")],
        modified: vec![PathBuf::from("unstaged_modified.txt")],
        deleted: vec![PathBuf::from("unstaged_deleted.txt")],
    };
    
    // Create a buffer to capture the output
    let mut output = Vec::new();
    
    // Call the output_porcelain function
    output_porcelain(&staged, &unstaged, &mut output);
    
    // Get the output as a string
    let output_str = String::from_utf8(output).unwrap();
    
    // Verify the output format
    let lines: Vec<&str> = output_str.trim().split('\n').collect();
    
    assert!(lines.contains(&"A  new_file.txt"));
    assert!(lines.contains(&"M  modified_file.txt"));
    assert!(lines.contains(&"D  deleted_file.txt"));
    assert!(lines.contains(&" M unstaged_modified.txt"));
    assert!(lines.contains(&" D unstaged_deleted.txt"));
    assert!(lines.contains(&"?? untracked_file.txt"));
}

#[tokio::test]
#[serial]
/// Tests the --porcelain flag for machine-readable output format.
/// Verifies that the output matches Git's porcelain format specification.
async fn test_status_porcelain() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create test data
    let mut file1 = fs::File::create("file1.txt").unwrap();
    file1.write_all(b"content").unwrap();
    
    let mut file2 = fs::File::create("file2.txt").unwrap();
    file2.write_all(b"content").unwrap();

    // Add one file to the staging area
    add::execute(AddArgs {
        pathspec: vec![String::from("file1.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
        refresh: false,
    }).await;

    // Add another file to the staging area and modify it
    add::execute(AddArgs {
        pathspec: vec![String::from("file2.txt")],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
        refresh: false,
    }).await;
    file2.write_all(b"modified content").unwrap();

    // Create a new file (untracked)
    let mut file3 = fs::File::create("file3.txt").unwrap();
    file3.write_all(b"new content").unwrap();

    // Create a buffer to capture the output
    let mut output = Vec::new();
    
    // Execute the status command with the --porcelain flag
    status_execute(StatusArgs { porcelain: true }, &mut output).await;
    
    // Get the output as a string
    let output_str = String::from_utf8(output).unwrap();
    
    // Verify the porcelain output format
    let lines: Vec<&str> = output_str.trim().split('\n').collect();

    // Should contain staged files
    assert!(lines.iter().any(|line| line.starts_with("A  file1.txt")));
    assert!(lines.iter().any(|line| line.starts_with("A  file2.txt")));
    // Should contain modified but unstaged files
    assert!(lines.iter().any(|line| line.starts_with(" M file2.txt")));
    
    // Should contain untracked files
    assert!(lines.iter().any(|line| line.starts_with("?? file3.txt")));
    
    // Should not contain human-readable text
    assert!(!output_str.contains("Changes to be committed"));
    assert!(!output_str.contains("Untracked files"));
    assert!(!output_str.contains("On branch"));
}
