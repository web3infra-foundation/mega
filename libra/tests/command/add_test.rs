use super::*;
use std::fs;
use std::io::Write;

#[tokio::test]
#[serial]
/// Tests the basic functionality of add command by adding a single file
async fn test_add_single_file() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a new file
    let file_content = "Hello, World!";
    let file_path = "test_file.txt";
    let mut file = fs::File::create(file_path).unwrap();
    file.write_all(file_content.as_bytes()).unwrap();

    // Execute add command
    add::execute(AddArgs {
        pathspec: vec![String::from(file_path)],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Verify the file was added to index
    let changes = changes_to_be_staged();
    assert!(!changes.new.iter().any(|x| x.to_str().unwrap() == file_path));
    assert!(changes.staged.iter().any(|x| x.to_str().unwrap() == file_path));
}

#[tokio::test]
#[serial]
/// Tests adding multiple files at once
async fn test_add_multiple_files() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create multiple files
    for i in 1..=3 {
        let file_content = format!("File content {}", i);
        let file_path = format!("test_file_{}.txt", i);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(file_content.as_bytes()).unwrap();
    }

    // Execute add command
    add::execute(AddArgs {
        pathspec: vec![
            String::from("test_file_1.txt"),
            String::from("test_file_2.txt"),
            String::from("test_file_3.txt"),
        ],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Verify all files were added to index
    let changes = changes_to_be_staged();
    assert!(changes.staged.iter().any(|x| x.to_str().unwrap() == "test_file_1.txt"));
    assert!(changes.staged.iter().any(|x| x.to_str().unwrap() == "test_file_2.txt"));
    assert!(changes.staged.iter().any(|x| x.to_str().unwrap() == "test_file_3.txt"));
}

#[tokio::test]
#[serial]
/// Tests the --all flag which adds all files in the working tree
async fn test_add_all_flag() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create multiple files
    for i in 1..=3 {
        let file_content = format!("File content {}", i);
        let file_path = format!("test_file_{}.txt", i);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(file_content.as_bytes()).unwrap();
    }

    // Execute add command with --all flag
    add::execute(AddArgs {
        pathspec: vec![],
        all: true,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Verify all files were added to index
    let changes = changes_to_be_staged();
    assert!(changes.staged.iter().any(|x| x.to_str().unwrap() == "test_file_1.txt"));
    assert!(changes.staged.iter().any(|x| x.to_str().unwrap() == "test_file_2.txt"));
    assert!(changes.staged.iter().any(|x| x.to_str().unwrap() == "test_file_3.txt"));
}

#[tokio::test]
#[serial]
/// Tests the --update flag which only updates files already in the index
async fn test_add_update_flag() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create files and add one to the index
    let tracked_file = "tracked_file.txt";
    let untracked_file = "untracked_file.txt";
    
    // Create and write initial content
    let mut file1 = fs::File::create(tracked_file).unwrap();
    file1.write_all(b"Initial content").unwrap();
    
    let mut file2 = fs::File::create(untracked_file).unwrap();
    file2.write_all(b"Initial content").unwrap();

    // Add only one file to the index
    add::execute(AddArgs {
        pathspec: vec![String::from(tracked_file)],
        all: false,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Modify both files
    let mut file1 = fs::OpenOptions::new().write(true).truncate(true).open(tracked_file).unwrap();
    file1.write_all(b" - Modified").unwrap();
    
    let mut file2 = fs::OpenOptions::new().write(true).open(untracked_file).unwrap();
    file2.write_all(b" - Modified").unwrap();

    // Execute add command with --update flag
    add::execute(AddArgs {
        pathspec: vec![String::from(".")],
        all: false,
        update: true,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Verify only tracked file was updated
    let changes = changes_to_be_staged();
    // Tracked file should appear in changes as modified (because it was updated)
    assert!(changes.modified.iter().any(|x| x.to_str().unwrap() == tracked_file));
    // Untracked file should still be untracked and show as new
    assert!(changes.new.iter().any(|x| x.to_str().unwrap() == untracked_file));
}

#[tokio::test]
#[serial]
/// Tests adding files with respect to ignore patterns in .libraignore
async fn test_add_with_ignore_patterns() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create .libraignore file
    let mut ignore_file = fs::File::create(".libraignore").unwrap();
    ignore_file.write_all(b"ignored_*.txt\nignore_dir/").unwrap();

    // Create files that should be ignored and not ignored
    let ignored_file = "ignored_file.txt";
    let tracked_file = "tracked_file.txt";
    
    // Create directory that should be ignored
    fs::create_dir("ignore_dir").unwrap();
    let ignored_dir_file = "ignore_dir/file.txt";

    // Create and write content
    let mut file1 = fs::File::create(ignored_file).unwrap();
    file1.write_all(b"Should be ignored").unwrap();
    
    let mut file2 = fs::File::create(tracked_file).unwrap();
    file2.write_all(b"Should be tracked").unwrap();
    
    let mut file3 = fs::File::create(ignored_dir_file).unwrap();
    file3.write_all(b"Should be ignored").unwrap();

    // Execute add command with all files
    add::execute(AddArgs {
        pathspec: vec![String::from(".")],
        all: true,
        update: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    // Verify only non-ignored files were added
    let changes = changes_to_be_staged();
    // Ignored files should not appear in changes.new
    assert!(!changes.new.iter().any(|x| x.to_str().unwrap() == ignored_file));
    // Directory files should not appear in changes.new
    assert!(!changes.new.iter().any(|x| x.to_str().unwrap() == ignored_dir_file));
    // Non-ignored file should not show as new (was added)
    assert!(!changes.new.iter().any(|x| x.to_str().unwrap() == tracked_file));
}

#[tokio::test]
#[serial]
/// Tests the dry-run flag which should not actually add files
async fn test_add_dry_run() {
    let test_dir = tempdir().unwrap();
    test::setup_with_new_libra_in(test_dir.path()).await;
    let _guard = test::ChangeDirGuard::new(test_dir.path());

    // Create a file
    let file_path = "test_file.txt";
    let mut file = fs::File::create(file_path).unwrap();
    file.write_all(b"Test content").unwrap();

    // Execute add command with dry-run
    add::execute(AddArgs {
        pathspec: vec![String::from(file_path)],
        all: false,
        update: false,
        verbose: false,
        dry_run: true,
        ignore_errors: false,
    })
    .await;

    // Verify the file was not actually added to index
    let changes = changes_to_be_staged();
    assert!(changes.new.iter().any(|x| x.to_str().unwrap() == file_path));
}
