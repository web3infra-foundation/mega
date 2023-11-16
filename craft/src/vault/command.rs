//!
//!
//!
//!
//!

use clap::{arg, Args, Subcommand};

use super::{
    crypt::{decrypt_blob, encrypt_blob, generate_key_full},
    init_rv_core,
    pgp_key::{delete_key, list_keys},
};

#[derive(Args, Debug)]
pub struct VaultArgs {
    #[command(subcommand)]
    mode: VaultMode,
}

#[derive(Clone, Subcommand, Debug)]
enum VaultMode {
    Newkey {
        #[arg(short, long)]
        id: Option<String>,

        #[arg(short, long)]
        path: String,
    },
    Encrypt {
        #[arg(short, long)]
        path: String,
    },
    Decrypt {
        #[arg(short, long)]
        path: String,
    },
    List,
    Delete {
        #[arg(short, long)]
        path: String,
    },
}

pub fn handle(args: VaultArgs) {
    let (core, token) = init_rv_core();
    // Match the mode with different functions

    match args.mode {
        // Generate key pair full to key_files and name it as your input
        VaultMode::Newkey { id, path } => {
            let primary_id = if let Some(id) = id {
                id
            } else {
                "User <craft@craft.com>".to_owned()
            };
            let _ = generate_key_full(&primary_id, &path, core, &token);
        }
        VaultMode::Encrypt { path } => {
            // Encrypt blob.data
            let _ = encrypt_blob(&path, core, &token);
        }
        VaultMode::Decrypt { path } => {
            // Decrypt blob.data
            let _ = decrypt_blob(&path, core, &token);
        }
        VaultMode::List => {
            // Show key lists and their fingerprint, key id.
            let _ = list_keys("secret/", core, &token);
        }
        VaultMode::Delete { path } => {
            // Delete key by key_name
            let _ = delete_key(&path, core, &token);
        }
    }
}

// Add a tests module with the # [cfg (test)] attribute
#[cfg(test)]
mod tests {

    use std::sync::{RwLock, Arc};

    use rusty_vault::core::Core;

    use crate::vault::{
        crypt::generate_key_full,
        init_rv_core,
        pgp_key::{delete_key, list_keys},
    };

    // Define a test function for generate-key-full mode
    // #[test]
    fn test_generate_key_full(core: Arc<RwLock<Core>>, token : &str) {
        // generate a full key
        let _ = generate_key_full("Craft <craft@craft.com>", "secret/craft", core, token);
    }

    // Define a test function for encrypt mode
    // #[test]
    fn test_encrypt(core: Arc<RwLock<Core>>, token : &str) {
        // generate key to crypt
        let _ = generate_key_full("User2 <sci@sci.com>", "secret/sci", core, token).unwrap();
        // Create and run a new process to execute the encrypt_blob function
        let mut child = std::process::Command::new("cargo")
            .arg("run")
            .arg("vault")
            .arg("encrypt")
            .arg("-p")
            .arg("secret/sci")
            .stdin(std::process::Stdio::piped()) // Pass the standard input stream as an argument
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        std::io::Write::write_all(
            &mut child.stdin.as_mut().unwrap(),
            b"This is a test message",
        )
        .expect("Failed to write to stdin");
        // Get the output of the child process
        let output = child.wait_with_output().expect("Failed to read stdout");
        // Check the output of the child process
        assert_eq!(output.status.code(), Some(0)); // The status code should be 0
        assert_eq!(output.stderr.len(), 0); // The standard error should be empty
                                            // Check if the contents are encrypted by looking for the PGP header
        assert!(output
            .stdout
            .starts_with("-----BEGIN PGP MESSAGE-----".as_bytes()));
    }

    // Define a test function for decrypt mode
    // #[test]
    fn test_decrypt(core: Arc<RwLock<Core>>, token : &str) {
        // Generate a key pair for testing
        let _ = generate_key_full(
            "User3 <basketball@basketball.com>",
            "secret/ball",
            core,
            token,
        );
        // Define the original content as a string
        let original_data = "This is a test message.";

        // Create a standard input stream from the string
        // Create and run a new process to execute the encrypt function
        let mut child_encrypt = std::process::Command::new("cargo")
            .arg("run")
            .arg("vault")
            .arg("encrypt")
            .arg("-p")
            .arg("secret/ball")
            .stdin(std::process::Stdio::piped()) // Pass the standard input stream as an argument
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        std::io::Write::write_all(
            &mut child_encrypt.stdin.as_mut().unwrap(),
            original_data.as_bytes(),
        )
        .expect("Failed to write to stdin");
        // Get the output of the child process
        let output_encrypt = child_encrypt
            .wait_with_output()
            .expect("Failed to read stdout");

        // Check the output of the child process
        assert_eq!(output_encrypt.status.code(), Some(0)); // The status code should be 0
        assert_eq!(output_encrypt.stderr.len(), 0); // The standard error should be empty

        // Create a standard input stream from the output of the encrypt function
        // Create and run a new process to execute the decrypt function
        let mut child_decrypt = std::process::Command::new("cargo")
            .arg("run")
            .arg("vault")
            .arg("decrypt")
            .arg("-p")
            .arg("secret/ball")
            .stdin(std::process::Stdio::piped()) // Pass the standard input stream as an argument
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        std::io::Write::write_all(
            &mut child_decrypt.stdin.as_mut().unwrap(),
            &output_encrypt.stdout,
        )
        .expect("Failed to write to stdin");
        // Get the output of the child process
        let output_decrypt = child_decrypt
            .wait_with_output()
            .expect("Failed to read stdout");

        // Check the output of the child process
        assert_eq!(output_decrypt.status.code(), Some(0)); // The status code should be 0
        assert_eq!(output_decrypt.stderr.len(), 0); // The standard error should be empty

        // Define the expected decrypted content as a string
        let expected_data = "This is a test message.";

        // Compare the output of the child process with the expected decrypted content
        assert_eq!(output_decrypt.stdout, expected_data.as_bytes()); // The standard output should match the expected string
    }

    // Define a test function for list-keys mode
    // #[test]
    fn test_list_keys(core: Arc<RwLock<Core>>, token : &str) {
        let actual = list_keys("secret/", core, token).unwrap();
        assert!(!actual.is_empty());
        // Check if the output contains the expected key information
    }

    // Define a test function for delete-key mode
    // #[test]
    fn test_delete_key(core: Arc<RwLock<Core>>, token : &str) {
        let _ = generate_key_full("Delete <delete@delete.com>", "secret/delete", core.clone(), token);
        let _ = delete_key("secret/delete", core.clone(), token);
    }

    #[test]
    fn test_basic_logical() {
        let (core, token) = init_rv_core();
        test_generate_key_full(core.clone(), &token);
        test_encrypt(core.clone(), &token);
        test_decrypt(core.clone(), &token);
        test_list_keys(core.clone(), &token);
        test_delete_key(core.clone(), &token);
    }
}
