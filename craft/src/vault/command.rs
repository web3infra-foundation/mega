//!
//!
//!
//!
//!

use std::path::PathBuf;

use clap::{arg, Args, Subcommand};

use crate::vault::{crypt::KeyOperation, init_rv_core, unseal_rv_core, RustyVault};

#[derive(Args, Debug)]
pub struct VaultArgs {
    #[command(subcommand)]
    mode: VaultMode,

    #[arg(short, long)]
    path: Option<PathBuf>,
}

#[derive(Clone, Subcommand, Debug, PartialEq)]
enum VaultMode {
    Init,
    Newkey {
        #[arg(short, long)]
        name: String,

        #[arg(long, default_value_t = String::from("aes-gcm"))]
        key_type: String,

        #[arg(short, long, default_value_t = 128)]
        bits: u32,
    },
    Encrypt {
        #[arg(short, long)]
        name: String,
    },
    Decrypt {
        #[arg(short, long)]
        name: String,
    },
    List,
    Delete {
        #[arg(short, long)]
        name: String,
    },
}

/// Handles different modes for interacting with the Rusty Vault.
///
/// It initializes the Rusty Vault Core and performs operations based on the specified mode.
///
/// # Arguments
///
/// * `args` - A VaultArgs enum representing different modes of operation.
pub fn handle(args: VaultArgs) {
    // Match the mode with different functions
    if args.mode == VaultMode::Init {
        init_rv_core(args.path.as_deref());
        let (core, token) = unseal_rv_core(args.path.as_deref());
        let rv = RustyVault { core, token };
        rv.mount_pki();
        // init a default ket with name craft
        let _ = rv.generate_key("craft", "aes-gcm", 128);
    } else {
        let (core, token) = unseal_rv_core(args.path.as_deref());
        let rv = RustyVault { core, token };
        match args.mode {
            // Generate key pair full to key_files and name it as your input
            VaultMode::Newkey {
                name,
                key_type,
                bits,
            } => {
                let _ = rv.generate_key(&name, &key_type, bits);
            }
            VaultMode::Encrypt { name } => {
                // Encrypt blob.data
                let _ = rv.encrypt(&name);
            }
            VaultMode::Decrypt { name } => {
                // Decrypt blob.data
                let _ = rv.decrypt(&name);
            }
            VaultMode::List => {
                // Show key lists and their fingerprint, key id.
                let _ = rv.list_keys("pki/keys");
            }
            VaultMode::Delete { name } => {
                // Delete key by key_name
                let _ = rv.delete_key(&name);
            }
            _ => panic!("Not Implement command"),
        }
    }
}

// Add a tests module with the # [cfg (test)] attribute
#[cfg(test)]
mod tests {

    use std::{env, fs, path::PathBuf};

    use go_defer::defer;

    use crate::vault::{crypt::KeyOperation, init_rv_core, unseal_rv_core, RustyVault};

    // Define a test function for generate-key-full mode
    // #[test]
    fn test_generate_key_full(rv: &RustyVault) {
        // generate a full key
        let _ = rv.generate_key("craft", "aes-gcm", 128);
    }

    // Define a test function for encrypt mode
    // #[test]
    fn test_encrypt(rv: &RustyVault, work_dir: &PathBuf) {
        // generate key to crypt
        let _ = rv.generate_key("sci", "aes-gcm", 128).unwrap();
        // Create and run a new process to execute the encrypt_blob function
        let mut child = std::process::Command::new("cargo")
            .arg("run")
            .arg("vault")
            .arg("-p")
            .arg(work_dir)
            .arg("encrypt")
            .arg("-n")
            .arg("sci")
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

        // // Check if the contents are encrypted by looking for the PGP header
        // assert!(output
        //     .stdout
        //     .starts_with("-----BEGIN PGP MESSAGE-----".as_bytes()));
    }

    // Define a test function for decrypt mode
    // #[test]
    fn test_decrypt(rv: &RustyVault, work_dir: &PathBuf) {
        // Generate a key pair for testing
        let _ = rv.generate_key("ball", "aes-gcm", 128);
        // Define the original content as a string
        let original_data = "This is a test message.";

        // Create a standard input stream from the string
        // Create and run a new process to execute the encrypt function
        let mut child_encrypt = std::process::Command::new("cargo")
            .arg("run")
            .arg("vault")
            .arg("-p")
            .arg(work_dir)
            .arg("encrypt")
            .arg("-n")
            .arg("ball")
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
            .arg("-p")
            .arg(work_dir)
            .arg("decrypt")
            .arg("-n")
            .arg("ball")
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
    // fn test_list_keys(rv: &RustyVault) {
    //     let actual = rv.list_keys("secret/").unwrap();
    //     assert!(!actual.is_empty());
    //     // Check if the output contains the expected key information
    // }

    // Define a test function for delete-key mode
    // #[test]
    // fn test_delete_key(rv: &RustyVault) {
    //     let _ = generate_key_full(
    //         "Delete <delete@delete.com>",
    //         "secret/delete",
    //         core.clone(),
    //         token,
    //     );
    //     let _ = delete_key("secret/delete", core.clone(), token);
    // }

    #[test]
    fn test_basic_logical() {
        // create a temporary directory for store config
        let temp = env::temp_dir().join("rusty_vault_core_init");
        defer! (
            assert!(fs::remove_dir_all(&temp).is_ok());
        );
        init_rv_core(Some(&temp));
        let (core, token) = unseal_rv_core(Some(&temp));
        let rv = RustyVault { core, token };
        rv.mount_pki();
        test_generate_key_full(&rv);
        test_encrypt(&rv, &temp);
        test_decrypt(&rv, &temp);
        // test_list_keys(&rv);
        // test_delete_key(&rv);
    }
}
