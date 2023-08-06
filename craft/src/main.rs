
use anyhow::{ Result, Ok};
use git_craft::{pgp_key::{list_keys, delete_key}, crypt::{generate_key, encrypt_blob, decrypt_blob}};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Keyargs {
    //accept mutiple values
    #[clap(num_args=1..,required=true)]
    file: Vec<String>,

}

fn main() -> Result<(), anyhow::Error> {

    //collect command line arguments into Args 
    let args =Keyargs::parse();
    // Check if there is no argument
    if args.file.is_empty() {
        // If not, print the usage information and exit
        println!("Available modes: generate-key, encrypt, decrypt, list-keys, delete-key");
        return Ok(());
    }

    // Get the first argument as the mode of operation
    let mode:&str =&args.file[0];
    // Match the mode with different functions
    match mode {
        // Generate key pair and save it to key_files
        "generate-key" => {
            // Generate key
            generate_key();
        }
        // Encrypt file contents with a public key
        "encrypt" => {
            // Encrypt blob.data
            let _ = encrypt_blob();
        }
        // Decrypt file contents with a secret key
        "decrypt" => {
            //Decrypt blob.data
            let _ =decrypt_blob();
        }
        "list-keys" => {
            //Show key lists
            let _ = list_keys();
        }
        "delete-key" => {
            //Delete key by fingerprint
            let _ =delete_key(&args.file[1]);
        }
        // For any other mode, print an error message and exit
        _ => {
            println!("Invalid mode: {}", mode);
            return Ok(());
        }

    }
     Ok(())
}

// Add a tests module with the # [cfg (test)] attribute
# [cfg (test)]
mod tests {
    use std::process::Command;

    // Import the names from outer scope
    use super::*;

    // Define a test function for generate-key mode
    # [test]
    fn test_generate_key() {
        // Create a mock argument vector with generate-key as the first element
        let mock_args = vec!["generate-key".to_string()];
        // Create a mock Keyargs struct from the mock argument vector
        //let mock_keyargs = Keyargs { file: mock_args };
        // Set a Mock ARGUMENT vector as an environment variable named AGRS
        std::env::set_var("ARGS", mock_args.join(" "));
        // Call the main function with the mock Keyargs struct and assert it returns Ok(())
        assert!(main().is_ok());
        // Check if the pub.asc and sec.asc files are created in the key_files directory
        assert!(std::path::Path::new("../craft/key_files/pub.asc").exists());
        assert!(std::path::Path::new("../craft/key_files/sec.asc").exists());
    }

    // Define a test function for encrypt mode
    # [test]
    fn test_encrypt() {
        // Create a mock argument vector with encrypt as the first element
        let mock_args = vec!["encrypt".to_string()];
        // Create a mock Keyargs struct from the mock argument vector
        //let mock_keyargs = Keyargs { file: mock_args };
        // Set a Mock ARGUMENT vector as an environment variable named AGRS
        std::env::set_var("ARGS", mock_args.join(" "));
        // Call the main function with the mock Keyargs struct and assert it returns Ok(())
        assert!(main().is_ok());
        // Read the contents of the message.txt file and assert it is not empty
        let message = std::fs::read_to_string("../craft/src/message.txt").unwrap();
        assert!(!message.is_empty());
        // Check if the contents are encrypted by looking for the PGP header
        assert!(message.starts_with("-----BEGIN PGP MESSAGE-----"));
    }

    // Define a test function for decrypt mode
    # [test]
    fn test_decrypt() {
        // Create a mock argument vector with decrypt as the first element
        let mock_args = vec!["decrypt".to_string()];
        // Create a mock Keyargs struct from the mock argument vector
        // Set a Mock ARGUMENT vector as an environment variable named AGRS
        std::env::set_var("ARGS", mock_args.join(" "));
        // Call the main function with the mock Keyargs struct and assert it returns Ok(())
        assert!(main().is_ok());
        // Read the contents of the message.txt file and assert it is not empty
        let message = std::fs::read_to_string("../craft/src/message.txt").unwrap();
        assert!(!message.is_empty());
        // Check if the contents are decrypted by looking for the plain text
        assert!(message.starts_with("This is a test message."));
    }

    // Define a test function for list-keys mode
    # [test]
    fn test_list_keys() {
        // Create a mock argument vector with list-keys as the first element
        let mock_args = vec!["list-keys".to_string()];
        // Create a mock Keyargs struct from the mock argument vector
        //let mock_keyargs = Keyargs { file: mock_args };
        // Set a Mock ARGUMENT vector as an environment variable named AGRS
        std::env::set_var("ARGS", mock_args.join(" "));
        // Call the main function with the mock Keyargs struct and assert it returns Ok(())
        assert!(main().is_ok());
        // Capture the standard output and assert it is not empty
        let output = Command::new(std::env::current_exe().unwrap())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute command");
        //To String
        let output = String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence");
        assert!(!output.is_empty());
        // Check if the output contains the expected key information
    }

    // Define a test function for delete-key mode
    # [test]
    fn test_delete_key() {
        // Create a mock argument vector with delete-key as the first element and a valid fingerprint as the second element
        let mock_args = vec!["delete-key F6B9C0F1E8A7D3B8C6E2E0F9A5A4D8C7B7C6D5A4".to_string()];
        // Set a Mock ARGUMENT vector as an environment variable named AGRS
        std::env::set_var("ARGS", mock_args.join(" "));
        // Call the main function with the mock Keyargs struct and assert it returns Ok(())
        assert!(main().is_ok());
        // Capture the standard output and assert it is not empty
        let output = Command::new(std::env::current_exe().unwrap())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute command");
        //To String
        let output = String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence");
        assert!(!output.is_empty());
    }
}