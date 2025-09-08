// Simple test program to verify rm --dry-run functionality
use libra::command::remove::{execute, RemoveArgs};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn main() {
    println!("Testing libra rm --dry-run functionality...");
    
    // Create a temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();
    
    // Create test files in the temporary directory
    let file1_path = temp_path.join("file1.txt");
    let file2_path = temp_path.join("file2.txt");
    
    let mut file1 = fs::File::create(&file1_path).unwrap();
    file1.write_all(b"Test content 1").unwrap();
    
    let mut file2 = fs::File::create(&file2_path).unwrap();
    file2.write_all(b"Test content 2").unwrap();
    
    println!("Created test files in temporary directory: {:?}", temp_path);
    
    // Test dry-run functionality
    let args = RemoveArgs {
        pathspec: vec![file1_path.to_string_lossy().to_string(), file2_path.to_string_lossy().to_string()],
        cached: false,
        recursive: false,
        force: true, // Use force mode to avoid requiring git repository
        dry_run: true,
    };
    
    println!("\nExecuting: libra rm --dry-run --force on test files");
    
    match execute(args) {
        Ok(_) => {
            println!("✓ dry-run executed successfully!");
            
            // Verify files still exist
            if file1_path.exists() && file2_path.exists() {
                println!("  Files still exist after dry-run (correct behavior)");
            } else {
                println!("  Error: dry-run should not actually delete files");
            }
        }
        Err(e) => {
            println!("✗ dry-run execution failed: {:?}", e);
        }
    }
    
    println!("\nTest completed, temporary directory will be automatically cleaned up");
    // The temporary directory will be automatically cleaned up when temp_dir goes out of scope
}
