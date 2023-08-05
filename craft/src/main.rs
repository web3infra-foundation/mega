use anyhow::{ Result, Context};
use git_craft::pgp_key::{generate_key_pair,encrypt_message, decrypt_message};
//use git::internal;
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
    //Usage
    println!("Available modes: generate-key , encrypt , decrypt");
    // Check if there is no argument
    if args.file.is_empty() {
        // If not, print the usage information and exit
        println!("Available modes: generate-key , encrypt , decrypt");
        return Ok(());
    }

    // Get the first argument as the mode of operation
    let mode:&str =&args.file[0];
    // Match the mode with different functions
    match mode {
        // Generate key pair and save it to key_files
        "generate-key" => {
            println!("Creating key pair, this will take a few seconds...");
            let key_pair = generate_key_pair().expect("Failed to generate key pair");
            //Generate a public key
            let pub_key = key_pair
                .public_key
                .to_armored_string(None)
                .expect("Failed to convert public key to armored ASCII string");
            //Write public key to pub.asc,it will replace the old public key
            _=std::fs::write( "../craft/key_files/pub.asc",pub_key).context("Writing public key to file");
            //Generate a secret key
            let sec_key = key_pair
                .secret_key
                .to_armored_string(None)
                .expect("Failed to convert secret key to armored ASCII string");
            //Write secret key and save it to sec.asc,same as public key
            _=std::fs::write( "../craft/key_files/sec.asc",sec_key).context("Writing secret key to file");
        }
        // Encrypt file contents with a public key
        "encrypt" => {
            // Usage if no mode
            if args.file.is_empty() {
                // If empty, print the usage information and exit
                println!("Usage: git-craft encrypt");
                return Ok(());
            }
            // Get the contents and the public key from file
            let msg =std::fs::read_to_string("../craft/src/message.txt").context("Reading message from file")?;
            // Encrypt the contents with the public key
            let encrypted = encrypt_message(&msg, "../craft/key_files/pub.asc").expect("Failed to encrypt message");
            //Print it to check whether it was encrypted
            println!("Encrypted: {}", encrypted);
        }
        // Decrypt file contents with a secret key
        "decrypt" => {
            //Print Usage if no mode
            if args.file.is_empty() {
                // Print the usage information and exit
                println!("Usage: git-craft decrypt");
                return Ok(());
            }
            // Get the encrypted file contents and the secret key from file
            let encrypted_msg =
            std::fs::read_to_string("../craft/src/encrypted_message.txt").context("Reading encrypted message from file")?;
            // Decrypt the message with the secret key
            let decrypted_msg = decrypt_message(encrypted_msg.as_str(),"../craft/key_files/sec.asc" ).expect("Failed to decrypt message");
            //Print decrypted message
            println!("Decrypted: {}", &decrypted_msg);
        }

        // For any other mode, print an error message and exit
        _ => {
            println!("Invalid mode: {}", mode);
            return Ok(());
        }

    }
     Ok(())
}
