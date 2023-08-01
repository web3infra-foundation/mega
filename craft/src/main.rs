use anyhow::{Context, Result};
use pgp::{composed, composed::signed_key::*, crypto, types::SecretKeyTrait, Deserializable};
use rand::prelude::*;
use smallvec::*;
use std::io::Cursor;
use std::env;


fn main() -> Result<(), anyhow::Error> {


    // Collect the command line arguments into a vector
    let args: Vec<String> = env::args().collect();

    // Check if there is at least one argument
    if args.len() <= 1 {
        // If not, print the usage information and exit
        println!("Usage: cargo run -- [mode]");
        println!("Available modes: generate-key , encrypt [message] [public key], decrypt [encrypted message] [secret key]");
        return Ok(());
    }

    // Get the first argument as the mode of operation
    let mode: &str = &args[1];

    // Match the mode with different functions
    match mode {
        // Generate a key pair and print it
        "generate-key" => {
            println!("Creating key pair, this will take a few seconds...");
            let key_pair = generate_key_pair().expect("Failed to generate key pair");
            let pub_key = key_pair
                .public_key
                .to_armored_string(None)
                .expect("Failed to convert public key to armored ASCII string");
            println!("Public key: {}", pub_key);
            let sec_key = key_pair
                .secret_key
                .to_armored_string(None)
                .expect("Failed to convert secret key to armored ASCII string");
            println!("Secret key: {}", sec_key);
        }
        // Encrypt a message with a public key
        "encrypt" => {
            // Check if there are two more arguments for the message and the public key
            if args.len() < 4 {
                // If not, print the usage information and exit
                println!("Usage: cargo run -- encrypt [message] [public_key]");
                return Ok(());
            }
            // Get the message and the public key from the arguments
            let msg = &args[2];
            let pub_key = &args[3];
            println!("Secret message: {}", msg);
            println!("Public key: {}", pub_key);
            // Encrypt the message with the public key
            let encrypted = encrypt(msg, pub_key).expect("Failed to encrypt message");
            println!("Encrypted: {}", encrypted);
        }
        // Decrypt a message with a secret key
        "decrypt" => {
            // Check if there are two more arguments for the encrypted message and the secret key
            if args.len() < 4 {
                // If not, print the usage information and exit
                println!("Usage: cargo run -- decrypt [encrypted_message] [secret_key]");
                return Ok(());
            }
            // Get the encrypted message and the secret key from the arguments
            let encrypted = &args[2];
            let sec_key = &args[3];
            println!("Encrypted message: {}", encrypted);
            println!("Secret key: {}", sec_key);
            // Parse the secret key from the string
            let (sec_key, _) = SignedSecretKey::from_string(sec_key)?;
            // Decrypt the message with the secret key
            let decrypted = decrypt(encrypted, &sec_key).expect("Failed to decrypt message");
            println!("Decrypted: {}", decrypted);
        }
        // For any other mode, print an error message and exit
        _ => {
            println!("Invalid mode: {}", mode);
            return Ok(());
        }
    }
     Ok(())
}


pub struct KeyPair {
    pub secret_key: pgp::SignedSecretKey,
    pub public_key: pgp::SignedPublicKey,
}

pub fn generate_key_pair() -> Result<KeyPair, anyhow::Error> {
   // let password = &args[2];

    let mut key_params = composed::key::SecretKeyParamsBuilder::default();
    key_params
        .key_type(composed::KeyType::Rsa(2048))
        //.passphrase(Some(password.clone()))
        .can_create_certificates(false)
        .can_sign(true)
        .primary_user_id("User <phyknife@phyknife.com>".into())
        .preferred_symmetric_algorithms(smallvec![crypto::sym::SymmetricKeyAlgorithm::AES256]);

    let secret_key_params = key_params
        .build()
        .expect("Must be able to create secret key params");

    let secret_key = secret_key_params
        .generate()
        .expect("Failed to generate a plain key.");

    let passwd_fn = String::new;
    let signed_secret_key = secret_key
        .sign(passwd_fn)
        .expect("Secret Key must be able to sign its own metadata");

    let public_key = signed_secret_key.public_key();
    let signed_public_key = public_key
        .sign(&signed_secret_key, passwd_fn)
        .expect("Public key must be able to sign its own metadata");

    let key_pair = KeyPair {
        secret_key: signed_secret_key,
        public_key: signed_public_key,
    };

    Ok(key_pair)
}

pub fn encrypt(msg: &str, pubkey_str: &str) -> Result<String, anyhow::Error> {
    let (pubkey, _) = SignedPublicKey::from_string(pubkey_str)?;
    // Requires a file name as the first arg, in this case I pass "none", as it's not used
    let msg = composed::message::Message::new_literal("none", msg);

    let mut rng = StdRng::from_entropy();
    let new_msg = msg.encrypt_to_keys(
        &mut rng,
        crypto::sym::SymmetricKeyAlgorithm::AES128,
        &[&pubkey],
    )?;
    Ok(new_msg.to_armored_string(None)?)
}

pub fn decrypt(armored: &str, seckey: &SignedSecretKey) -> Result<String, anyhow::Error> {
    let buf = Cursor::new(armored);
    let (msg, _) = composed::message::Message::from_armor_single(buf)
        .context("Failed to convert &str to armored message")?;
    let (decryptor, _) = msg
        .decrypt(|| String::from(""), || String::from(""), &[seckey])
        .context("Decrypting the message")?;

    for msg in decryptor {
        let bytes = msg?.get_content()?.unwrap();
        let clear_text = String::from_utf8(bytes)?;
        if String::len(&clear_text) > 0 {
            return Ok(clear_text);
        }
    }

    Err(anyhow::Error::msg("Failed to find message"))
}
