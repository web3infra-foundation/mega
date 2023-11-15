//!
//!
//!
//!
//!
use std::{
    fs,
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::{Ok, Result};
use clap::Parser;

use git_craft::{
    crypt::{decrypt_blob, encrypt_blob, generate_key, generate_key_full},
    pgp_key::{delete_key, list_keys},
};

use rusty_vault::{
    cli::config,
    core::{Core, InitResult, SealConfig},
    http::sys::InitResponse,
    storage::{barrier_aes_gcm, physical},
};

#[derive(Parser)]
#[command(author = "Jiajun Li <frankanepc@gmail.com>", version = "0.1.0")]
#[command(
    about = "Git encrypt tool",
    long_about = "Usage: generate-key, generate-key-full [primary_id] [key_name], encrypt [public_key_path], decrypt [secret_key_path], list-keys , delete-key [key_name]"
)]

struct CraftOptions {
    //accept mutiple values, it needs 1 value at least
    #[clap(num_args=1..,required=true)]
    command: Vec<String>,
}

// Program main function
// Arguments: accept command line arguments.
fn main() -> Result<(), anyhow::Error> {
    // Collect command line arguments into Args
    let args = CraftOptions::parse();
    // Check if there is no argument
    if args.command.is_empty() {
        // If not, print the usage information and exit
        println!("Available modes: generate-key, generate-key-full [primary_id] [key_name], encrypt [public_key_path], decrypt [secret_key_path], list-keys [key_path], delete-key [key_name] [key_path]");
        return Ok(());
    }
    let core = init_rv_core();
    // Get the first argument as the mode of operation
    let mode: &str = &args.command[0];
    // Match the mode with different functions
    match mode {
        // Generate default key pair and save it to key_files
        "generate-key" => {
            // Generate key
            let _ = generate_key(core);
        }
        // Generate key pair full to key_files and name it as your input
        "generate-key-full" => {
            // Generate a full key
            let _ = generate_key_full(&args.command[1], &args.command[2], core);
        }
        // Encrypt file contents with a public key
        "encrypt" => {
            // Encrypt blob.data
            let _ = encrypt_blob(&args.command[1], core);
        }
        // Decrypt file contents with a secret key
        "decrypt" => {
            // Decrypt blob.data
            let _ = decrypt_blob(&args.command[1], core);
        }
        "list-keys" => {
            // Show key lists and their fingerprint, key id.
            let _ = list_keys("secret/", core);
        }
        "delete-key" => {
            // Delete key by key_name
            let _ = delete_key(&args.command[1], core);
        }
        // For any other mode, print an error message and exit
        _ => {
            println!("Invalid mode: {}", mode);
            return Ok(());
        }
    }
    Ok(())
}

pub const WORK_DIR_PATH_DEFAULT: &str = "/tmp/.mega/rusty_vault";

pub fn init_rv_core() -> Arc<RwLock<Core>> {
    let path = Path::new(WORK_DIR_PATH_DEFAULT);
    let config_path = path.join("config.hcl");
    let secrets_path = path.join("secrets");
    if !path.exists() {
        fs::create_dir_all(WORK_DIR_PATH_DEFAULT).unwrap();
    }

    if !config_path.exists() {
        let hcl_config = r#"
        storage "file" {
          path    = "/tmp/.mega/rusty_vault/data"
        }

        listener "tcp" {
          address     = "127.0.0.1:8200"
        }

        api_addr = "http://127.0.0.1:8200"
        log_level = "debug"
        log_format = "{date} {req.path}"
        pid_file = "/tmp/rusty_vault.pid"
    "#;
        fs::write(&config_path, hcl_config).unwrap();
    }
    let config = config::load_config(config_path.to_str().unwrap()).unwrap();

    let (_, storage) = config.storage.iter().next().unwrap();

    let backend = physical::new_backend(storage.stype.as_str(), &storage.config).unwrap();

    let barrier = barrier_aes_gcm::AESGCMBarrier::new(Arc::clone(&backend));

    let c = Arc::new(RwLock::new(Core {
        physical: backend,
        barrier: Arc::new(barrier),
        ..Default::default()
    }));

    {
        let mut core = c.write().unwrap();
        core.self_ref = Some(Arc::clone(&c));

        let seal_config = SealConfig {
            secret_shares: 10,
            secret_threshold: 5,
        };

        let mut unsealed = false;
        let init_result = if core.inited().unwrap() {
            let init_response: InitResponse =
                serde_json::from_str(&fs::read_to_string(secrets_path).unwrap()).unwrap();
            let init_result = InitResult {
                secret_shares: init_response
                    .keys
                    .iter()
                    .map(|key| hex::decode(key).unwrap())
                    .collect(),
                root_token: init_response.root_token,
            };
            core.module_manager.init(&core).unwrap();
            init_result
        } else {
            let result = core.init(&seal_config);
            assert!(result.is_ok());
            let init_result = result.unwrap();
            let resp = InitResponse {
                keys: init_result
                    .secret_shares
                    .iter()
                    .map(hex::encode)
                    .collect(),
                root_token: init_result.root_token.clone(),
            };
            // TODO: need to find a way to secure preserve key later
            fs::write(secrets_path, serde_json::to_string(&resp).unwrap()).unwrap();
            init_result
        };
        for i in 0..seal_config.secret_threshold {
            let key = &init_result.secret_shares[i as usize];
            let unseal = core.unseal(key);
            assert!(unseal.is_ok());
            unsealed = unseal.unwrap();
        }
        assert!(unsealed);
    }
    Arc::clone(&c)
}

// Add a tests module with the # [cfg (test)] attribute
#[cfg(test)]
mod tests {

    // Import the names from outer scope
    use super::*;

    // Define a test function for generate-key mode
    #[test]
    fn test_generate_key() {
        // Generate key
        let core = init_rv_core();
        let _ = generate_key(core);
        // Check if the pub.asc and sec.asc files are created in the key_files directory
        // assert!(std::path::Path::new("../craft/key_files/pub.asc").exists());
        // assert!(std::path::Path::new("../craft/key_files/sec.asc").exists());
    }

    // Define a test function for generate-key-full mode
    #[test]
    fn test_generate_key_full() {
        let core = init_rv_core();
        // generate a full key
        let _ = generate_key_full("User1 <goodman@goodman.com>", "goodman", core);
        // check if the goodman.asc and sec.asc files are created in the default key file directory
        // assert!(std::path::Path::new("../craft/key_files/goodmanpub.asc").exists());
        // assert!(std::path::Path::new("../craft/key_files/goodmanpub.asc").exists());
    }

    // Define a test function for encrypt mode
    #[test]
    fn test_encrypt() {
        let core = init_rv_core();
        // generate key to crypt
        let _ = generate_key_full("User2 <sci@sci.com>", "sci", core).unwrap();
        // Create and run a new process to execute the encrypt_blob function
        let mut child = std::process::Command::new("cargo")
            .arg("run")
            .arg("encrypt")
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
    #[test]
    fn test_decrypt() {
        let core = init_rv_core();
        // Generate a key pair for testing
        let _ = generate_key_full("User3 <basketball@basketball.com>", "ball", core);
        // Define the original content as a string
        let original_data = "This is a test message.";

        // Create a standard input stream from the string
        // Create and run a new process to execute the encrypt function
        let mut child_encrypt = std::process::Command::new("cargo")
            .arg("run")
            .arg("encrypt")
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
            .arg("decrypt")
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
    #[test]
    fn test_list_keys() {
        let core = init_rv_core();
        let actual = list_keys("secret/", core).unwrap();
        assert!(!actual.is_empty());
        // Check if the output contains the expected key information
    }

    // Define a test function for delete-key mode
    #[test]
    fn test_delete_key() {
        let core = init_rv_core();
        let _ = generate_key_full("Delete <delete@delete.com>", "delete", core.clone());
        let _ = delete_key("secret/delete", core.clone());
    }
}
