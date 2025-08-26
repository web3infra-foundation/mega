use super::*;
use std::cmp::min;
use clap::Parser;
use mercury::internal::object::commit::Commit;
#[tokio::test]
#[serial]
/// Tests retrieval of commits reachable from a specific commit hash
async fn test_get_reachable_commits() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = test::ChangeDirGuard::new(temp_path.path());

    let commit_id = create_test_commit_tree().await;

    let reachable_commits = get_reachable_commits(commit_id).await;
    assert_eq!(reachable_commits.len(), 6);
}

#[tokio::test]
#[serial]
/// Tests log command execution functionality
async fn test_execute_log() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());
    let _ = create_test_commit_tree().await;

    // let args = LogArgs { number: Some(1) };
    // execute(args).await;
    let head = Head::current().await;
    // check if the current branch has any commits
    if let Head::Branch(branch_name) = head.to_owned() {
        let branch = Branch::find_branch(&branch_name, None).await;
        if branch.is_none() {
            panic!("fatal: your current branch '{branch_name}' does not have any commits yet ");
        }
    }

    let commit_hash = Head::current_commit().await.unwrap().to_string();

    let mut reachable_commits = get_reachable_commits(commit_hash.clone()).await;
    // default sort with signature time
    reachable_commits.sort_by(|a, b| b.committer.timestamp.cmp(&a.committer.timestamp));
    //the last seven commits
    let max_output_number = min(6, reachable_commits.len());
    let mut output_number = 6;
    for commit in reachable_commits.iter().take(max_output_number) {
        assert_eq!(commit.message, format!("\nCommit_{output_number}"));
        output_number -= 1;
    }
}

/// create a test commit tree structure as graph and create branch (master) head to commit 6
/// return a commit hash of commit 6
///            3 --  6
///          /      /
///    1 -- 2  --  5
//           \   /   \
///            4     7
async fn create_test_commit_tree() -> String {
    let mut commit_1 = Commit::from_tree_id(
        SHA1::new(&[1; 20]),
        vec![],
        &format_commit_msg("Commit_1", None),
    );
    commit_1.committer.timestamp = 1;
    // save_object(&commit_1);
    save_object(&commit_1, &commit_1.id).unwrap();

    let mut commit_2 = Commit::from_tree_id(
        SHA1::new(&[2; 20]),
        vec![commit_1.id],
        &format_commit_msg("Commit_2", None),
    );
    commit_2.committer.timestamp = 2;
    save_object(&commit_2, &commit_2.id).unwrap();

    let mut commit_3 = Commit::from_tree_id(
        SHA1::new(&[3; 20]),
        vec![commit_2.id],
        &format_commit_msg("Commit_3", None),
    );
    commit_3.committer.timestamp = 3;
    save_object(&commit_3, &commit_3.id).unwrap();

    let mut commit_4 = Commit::from_tree_id(
        SHA1::new(&[4; 20]),
        vec![commit_2.id],
        &format_commit_msg("Commit_4", None),
    );
    commit_4.committer.timestamp = 4;
    save_object(&commit_4, &commit_4.id).unwrap();

    let mut commit_5 = Commit::from_tree_id(
        SHA1::new(&[5; 20]),
        vec![commit_2.id, commit_4.id],
        &format_commit_msg("Commit_5", None),
    );
    commit_5.committer.timestamp = 5;
    save_object(&commit_5, &commit_5.id).unwrap();

    let mut commit_6 = Commit::from_tree_id(
        SHA1::new(&[6; 20]),
        vec![commit_3.id, commit_5.id],
        &format_commit_msg("Commit_6", None),
    );
    commit_6.committer.timestamp = 6;
    save_object(&commit_6, &commit_6.id).unwrap();

    let mut commit_7 = Commit::from_tree_id(
        SHA1::new(&[7; 20]),
        vec![commit_5.id],
        &format_commit_msg("Commit_7", None),
    );
    commit_7.committer.timestamp = 7;
    save_object(&commit_7, &commit_7.id).unwrap();

    // set current branch head to commit 6
    let head = Head::current().await;
    let branch_name = match head {
        Head::Branch(name) => name,
        _ => panic!("should be branch"),
    };

    Branch::update_branch(&branch_name, &commit_6.id.to_string(), None).await;

    commit_6.id.to_string()
}

#[tokio::test]
#[serial]
/// Tests log command with --oneline parameter
async fn test_log_oneline() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // Create test commits
    let commit_id = create_test_commit_tree().await;
    let reachable_commits = get_reachable_commits(commit_id).await;

    // Test oneline format
    let args = LogArgs::try_parse_from(["libra", "--number", "3", "--oneline"]);

    // Since execute function writes to stdout, we'll test the logic directly
    let mut sorted_commits = reachable_commits.clone();
    sorted_commits.sort_by(|a, b| b.committer.timestamp.cmp(&a.committer.timestamp));

    let max_commits = std::cmp::min(args.unwrap().number.unwrap_or(usize::MAX), sorted_commits.len());

    for (i, commit) in sorted_commits.iter().take(max_commits).enumerate() {
        // Test short hash format (should be 7 characters)
        let short_hash = &commit.id.to_string()[..7];
        assert_eq!(short_hash.len(), 7);

        // Test that commit message parsing works
        let (msg, _) = common::utils::parse_commit_msg(&commit.message);
        assert!(!msg.is_empty());

        // For our test commits, verify the expected format
        let expected_number = 6 - i; // commits are numbered 6, 5, 4, 3, 2, 1
        assert_eq!(msg.trim(), format!("Commit_{expected_number}"));
    }
}


#[tokio::test]
#[serial]
/// Tests log -p (patch) without pathspec: create A -> commit -> create B -> commit -> assert diffs contain both A and B contents
async fn test_log_patch_no_pathspec() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // Create file A and commit
    test::ensure_file("A.txt", Some("Content A\n"));
    add::execute(AddArgs {
        pathspec: vec![String::from("A.txt")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "Add A".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    // Create file B and commit
    test::ensure_file("B.txt", Some("Content B\n"));
    add::execute(AddArgs {
        pathspec: vec![String::from("B.txt")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;
    commit::execute(CommitArgs {
        message: "Add B".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    let bin_dir = temp_path.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let out_file = temp_path.path().join("less_out.txt");

    if cfg!(windows) {
        // Windows: use PowerShell to fetch stdin
        let ps_script = format!(
            r#"
$content = [Console]::In.ReadToEnd()
[System.IO.File]::WriteAllText('{}', $content)
"#,
            out_file.display().to_string().replace("\\", "\\\\")
        );
        let ps_path = bin_dir.join("less.ps1");
        std::fs::write(&ps_path, ps_script.as_bytes()).unwrap();

        // Create a batch file to call PowerShell
        let bat_path = bin_dir.join("less.bat");
        let bat_script = format!(
            "@echo off\r\npowershell -ExecutionPolicy Bypass -File \"{}\"\r\n",
            ps_path.display()
        );
        std::fs::write(&bat_path, bat_script.as_bytes()).unwrap();
    } else {
        // Unix: use shell script
        let less_path = bin_dir.join("less");
        let script = format!("#!/bin/sh\ncat - > \"{}\"\n", out_file.display());
        std::fs::write(&less_path, script.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&less_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    // Set PATH
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = if cfg!(windows) {
        format!("{};{}", bin_dir.display(), old_path)
    } else {
        format!("{}:{}", bin_dir.display(), old_path)
    };
    std::env::set_var("PATH", &new_path);

    // Call log command
    let args = LogArgs::try_parse_from(["libra", "--number", "2", "-p"]).unwrap();
    libra::command::log::execute(args).await;

    // Restore PATH
    std::env::set_var("PATH", old_path);

    let combined_out = std::fs::read_to_string(&out_file).unwrap_or_default();
    assert!(combined_out.contains("Content A"), "patch should contain A content, got: {}", combined_out);
    assert!(combined_out.contains("Content B"), "patch should contain B content, got: {}", combined_out);
}

#[tokio::test]
#[serial]
/// Tests log -p with a specific pathspec: commit contains A and B, but log -p A should only include A
async fn test_log_patch_with_pathspec() {
    let temp_path = tempdir().unwrap();
    test::setup_with_new_libra_in(temp_path.path()).await;
    let _guard = ChangeDirGuard::new(temp_path.path());

    // Create files A and B and commit both in one commit
    test::ensure_file("A.txt", Some("Content A\n"));
    test::ensure_file("B.txt", Some("Content B\n"));

    add::execute(AddArgs {
        pathspec: vec![String::from(".")],
        all: false,
        update: false,
        refresh: false,
        verbose: false,
        dry_run: false,
        ignore_errors: false,
    })
    .await;

    commit::execute(CommitArgs {
        message: "Add A and B".to_string(),
        allow_empty: false,
        conventional: false,
        amend: false,
        signoff: false,
        disable_pre: false,
    })
    .await;

    let bin_dir = temp_path.path().join("bin2");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let out_file = temp_path.path().join("less_out_pathspec.txt");

    if cfg!(windows) {
        let ps_script = format!(
            r#"
$content = [Console]::In.ReadToEnd()
[System.IO.File]::WriteAllText('{}', $content)
"#,
            out_file.display().to_string().replace("\\", "\\\\")
        );
        let ps_path = bin_dir.join("less.ps1");
        std::fs::write(&ps_path, ps_script.as_bytes()).unwrap();
        
        let bat_path = bin_dir.join("less.bat");
        let bat_script = format!(
            "@echo off\r\npowershell -ExecutionPolicy Bypass -File \"{}\"\r\n",
            ps_path.display()
        );
        std::fs::write(&bat_path, bat_script.as_bytes()).unwrap();
    } else {
        let less_path = bin_dir.join("less");
        let script = format!("#!/bin/sh\ncat - > \"{}\"\n", out_file.display());
        std::fs::write(&less_path, script.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&less_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    // Set PATH
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = if cfg!(windows) {
        format!("{};{}", bin_dir.display(), old_path)
    } else {
        format!("{}:{}", bin_dir.display(), old_path)
    };
    std::env::set_var("PATH", &new_path);

    // Call log command
    let args = LogArgs::try_parse_from(["libra", "-p", "A.txt"]).unwrap();
    libra::command::log::execute(args).await;

    std::env::set_var("PATH", old_path);

    let out = std::fs::read_to_string(out_file).unwrap_or_default();
    assert!(out.contains("Content A"), "patch should contain A content, got: {}", out);
    assert!(!out.contains("Content B"), "patch should not contain B content when pathspec is A, got: {}", out);
}
