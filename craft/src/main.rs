
use anyhow::{ Result, Ok};
use git_craft::{pgp_key::{list_keys, delete_key}, crypt::{generate_key, encrypt_blob, decrypt_blob, generate_key_full}};
use clap::Parser;

#[derive(Parser)]
#[command(author = "Jiajun Li <frankanepc@gmail.com>", version = "0.1.0", about = "Git crypt tool", long_about=None)]
struct CraftOptions {
    //accept mutiple values, it needs 1 value at least
    #[clap(num_args=1..,required=true)]
    command: Vec<String>,
}

// Program main function
// Arguments: accept command line arguments.
fn main() -> Result<(), anyhow::Error> {

    // Collect command line arguments into Args 
    let args =CraftOptions::parse();
    // Check if there is no argument
    if args.command.is_empty() {
        // If not, print the usage information and exit
        println!("Available modes: generate-key, generate-key-full [primary_id] [key_name], encrypt [file_path] [public_key_path], decrypt [secret_key_path], list-keys [key_path], delete-key [key_name] [key_path]");
        return Ok(());
    }

    // Get the first argument as the mode of operation
    let mode:&str =&args.command[0];
    // Match the mode with different functions
    match mode {
        // Generate default key pair and save it to key_files
        "generate-key" => {
            // Generate key
            let _ = generate_key();
        }
        // Generate key pair full to key_files and name it as your input
        "generate-key-full" => {
            // Generate a full key
            let _ = generate_key_full(&args.command[1], &args.command[2]);
        }
        // Encrypt file contents with a public key
        "encrypt" => {
            // Encrypt blob.data
            let _ = encrypt_blob(&args.command[1],"../craft/key_files/pub.asc");
        }
        // Decrypt file contents with a secret key
        "decrypt" => {
            // Decrypt blob.data
            let _ =decrypt_blob("../craft/key_files/sec.asc");
        }
        "list-keys" => {
            // Show key lists and their fingerprint, key id.
            let _ = list_keys("../craft/key_files");
        }
        "delete-key" => {
            // Delete key by key_name
            let _ =delete_key(&args.command[1], "../craft/key_files");
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
    
    // Import the names from outer scope
    use super::*;
    
    
    // Define a test function for generate-key mode
    # [test]
    fn test_generate_key() {
        // Generate key
        let _ = generate_key();
        // Check if the pub.asc and sec.asc files are created in the key_files directory
        assert!(std::path::Path::new("../craft/key_files/pub.asc").exists());
        assert!(std::path::Path::new("../craft/key_files/sec.asc").exists());
    } 

    // Define a test function for generate-key-full mode
    #[test]
    fn test_generate_key_full() {
        // generate a full key
        let _ = generate_key_full("User1 <goodman@goodman.com>", "goodman");
        // check if the goodman.asc and sec.asc files are created in the default key file directory
        assert!(std::path::Path::new("../craft/key_files/goodmanpub.asc").exists());
        assert!(std::path::Path::new("../craft/key_files/goodmanpub.asc").exists());
    } 

    // Define a test function for encrypt mode
    # [test]
    fn test_encrypt() {
        // generate key to crypt
        let _ = generate_key();
        // encrypt test contents
        let _ = encrypt_blob("../tests/data/objects/message.txt","../craft/key_files/pub.asc");
        // Read the contents of the message.txt file and assert it is not empty
        let message = std::fs::read_to_string("../tests/data/objects/message.txt").unwrap();
        std::fs::write("../tests/objects/encrypt.txt", message).expect("Unable to write test encrypt output");
        let message = std::fs::read_to_string("../tests/objects/encrypt.txt").unwrap();
        assert!(!message.is_empty());
        // Check if the contents are encrypted by looking for the PGP header
        assert!(message.starts_with("-----BEGIN PGP MESSAGE-----"));
        //Decrypt it to do next test
        //let _ = decrypt_blob("../tests/data/objects/message.txt","../craft/key_files/sec.asc");
    }

    // Define a test function for decrypt mode
    # [test]
    fn test_decrypt_blob() {
        let _ = generate_key();
        let _ = encrypt_blob("../tests/data/objects/emessage.txt","../craft/key_files/pub.asc");
        
        // Read the file content and convert it to a vector of bytes
        let vec = std::fs::read("../tests/data/objects/emessage.txt").expect("Failed to read file");

        // Create a standard input stream from the vector of bytes
        // Create and run a new process to execute the decrypt_blob function
        let mut child = std::process::Command::new("cargo")
            .arg("run")
            .arg("decrypt")
            .stdin(std::process::Stdio::piped()) // Pass the standard input stream as an argument
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");
        
        std::io::Write::write_all(&mut child.stdin.as_mut().unwrap(), &vec).expect("Failed to write to stdin");    
        // Get the output of the child process
        let output = child.wait_with_output().expect("Failed to read stdout");

        // Check the output of the child process
        assert_eq!(output.status.code(), Some(0)); // The status code should be 0
        assert_eq!(output.stderr.len(), 0); // The standard error should be empty

        // Define the expected decrypted content as a string
        let expected_data = "This is a test message.";
    
        // Compare the output of the child process with the expected decrypted content
        assert_eq!(output.stdout, expected_data.as_bytes()); // The standard output should match the expected string
}

    // Define a test function for list-keys mode
    # [test]
    fn test_list_keys() {
        let _ = generate_key();
        let actual = list_keys("../craft/key_files").unwrap();
        assert!(!actual.is_empty());
        // Check if the output contains the expected key information
    }

    // Define a test function for delete-key mode
    # [test]
    fn test_delete_key() {
        let _ = generate_key();
        let _ = delete_key("", "../craft/key_files");
        assert!(!std::path::Path::new("../craft/key_files/pub.asc").exists());
        assert!(!std::path::Path::new("../craft/key_files/sec.asc").exists());
    } 
}
