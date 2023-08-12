
use anyhow::{ Result, Ok};
use git_craft::{pgp_key::{list_keys, delete_key}, crypt::{generate_key, encrypt_blob, decrypt_blob, generate_key_full}};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Keyargs {
    //accept mutiple values, it needs 1 value at least
    #[clap(num_args=1..,required=true)]
    file: Vec<String>,
}

// Program main function
// Arguments: accept command line arguments, itself does not need arguments.
fn main() -> Result<(), anyhow::Error> {

    // Collect command line arguments into Args 
    let args =Keyargs::parse();
    // Check if there is no argument
    if args.file.is_empty() {
        // If not, print the usage information and exit
        println!("Available modes: generate-key, generate-key-full [primary_id] [key_name], encrypt [file_path] [public_key_path], decrypt [secret_key_path], list-keys [key_path], delete-key [key_name] [key_path]");
        return Ok(());
    }

    // Get the first argument as the mode of operation
    let mode:&str =&args.file[0];
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
            let _ = generate_key_full(&args.file[1], &args.file[2]);
        }
        // Encrypt file contents with a public key
        "encrypt" => {
            // Encrypt blob.data
            let _ = encrypt_blob(&args.file[1],"/mega/craft/key_files/pub.asc");
        }
        // Decrypt file contents with a secret key
        "decrypt" => {
            // Decrypt blob.data
            let _ =decrypt_blob("/mega/craft/key_files/sec.asc");
        }
        "list-keys" => {
            // Show key lists
            let _ = list_keys("/mega/craft/key_files");
        }
        "delete-key" => {
            // Delete key by key_id
            let _ =delete_key(&args.file[1], "/mega/craft/key_files");
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
        assert!(std::path::Path::new("/mega/craft/key_files/pub.asc").exists());
        assert!(std::path::Path::new("/mega/craft/key_files/sec.asc").exists());
    }
    /* 
    // Define a test function for encrypt mode
    # [test]
    fn test_encrypt() {
        generate_key();
        // Create a mock argument vector with encrypt as the first element
        let _ = encrypt_blob("../tests/data/objects/message.txt","../craft/key_files/pub.asc");
        // Read the contents of the message.txt file and assert it is not empty
        let message = std::fs::read_to_string("../tests/data/objects/message.txt").unwrap();
        std::fs::write("../tests/objects/encrypt.txt", message).expect("Unable to write test encrypt output");
        let message = std::fs::read_to_string("../tests/objects/encrypt.txt").unwrap();
        assert!(!message.is_empty());
        // Check if the contents are encrypted by looking for the PGP header
        assert!(message.starts_with("-----BEGIN PGP MESSAGE-----"));
        //Decrypt it to do next test
        let _ = decrypt_blob("../tests/data/objects/message.txt","../craft/key_files/sec.asc");
    }

    // Define a test function for decrypt mode
    # [test]
    fn test_decrypt() {
        generate_key();
        let _ = encrypt_blob("../tests/data/objects/message.txt","../craft/key_files/pub.asc");
        let _ = decrypt_blob("../tests/data/objects/message.txt","../craft/key_files/sec.asc");
        // Read the contents of the message.txt file and assert it is not empty
        let message = std::fs::read_to_string("../tests/data/objects/message.txt").unwrap();
        std::fs::write("../tests/objects/decrypt.txt", message).expect("Unable to write test encrypt output");
        let message = std::fs::read_to_string("../tests/objects/decrypt.txt").unwrap();
        assert!(!message.is_empty());
        // Check if the contents are decrypted by looking for the plain text
        assert!(message.starts_with("This is a test message."));
    }
    
    // Define a test function for list-keys mode
    # [test]
    fn test_list_keys() {
        generate_key();
        let actual = list_keys("../craft/key_files/pub.asc","../craft/key_files/sec.asc").unwrap();
        assert!(!actual.is_empty());
        // Check if the output contains the expected key information
    }

    // Define a test function for delete-key mode
    # [test]
    fn test_delete_key() {
        generate_key();
        // Create a mock argument vector with delete-key as the first element and a valid fingerprint as the second element
        let data = delete_key("../craft/key_files/pub.asc","../craft/key_files/sec.asc").unwrap();
        // Capture the standard output and assert it is not empty
        assert!(!data.is_empty());
    }*/ 
}
