// Simple test program to verify rm --dry-run functionality
use libra::command::remove::{execute, RemoveArgs};
use std::fs;
use std::io::Write;

fn main() {
    println!("Testing libra rm --dry-run functionality...");
    
    // Create test files
    fs::create_dir_all("test_files").unwrap();
    let mut file1 = fs::File::create("test_files/file1.txt").unwrap();
    file1.write_all(b"Test content 1").unwrap();
    
    let mut file2 = fs::File::create("test_files/file2.txt").unwrap();
    file2.write_all(b"Test content 2").unwrap();
    
    println!("Created test files: test_files/file1.txt, test_files/file2.txt");
    
    // Test dry-run functionality
    let args = RemoveArgs {
        pathspec: vec!["test_files/file1.txt".to_string(), "test_files/file2.txt".to_string()],
        cached: false,
        recursive: false,
        force: true, // Use force mode to avoid requiring git repository
        dry_run: true,
    };
    
    println!("\nExecuting: libra rm --dry-run --force test_files/file1.txt test_files/file2.txt");
    
    match execute(args) {
        Ok(_) => {
            println!("✓ dry-run executed successfully!");
            
            // Verify files still exist
            if fs::metadata("test_files/file1.txt").is_ok() && fs::metadata("test_files/file2.txt").is_ok() {
                println!("  Files still exist after dry-run (correct behavior)");
            } else {
                println!("  Error: dry-run should not actually delete files");
            }
        }
        Err(e) => {
            println!("✗ dry-run execution failed: {:?}", e);
        }
    }
    
    // Clean up test files
    let _ = fs::remove_dir_all("test_files");
    println!("\nTest completed, test files cleaned up");
}
